use std::fs::File;
use std::os::fd::AsRawFd;
use std::io::{Error, Result};

// https://arm64.syscall.sh/
// fn sys_flock(fd: i32, operation: i32) -> i32 {
//     let res: i32;
//     unsafe {
//         asm!(
//             "svc 0",
//             in("x8") 0x20,
//             in("x0") fd,
//             in("x1") operation,
//             lateout("x0") res,
//             clobber_abi("C"),
//         );
//     }
// 	res
// }

extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

const LOCK_SH : i32 = 1;
const LOCK_EX : i32 = 2;
const LOCK_NB : i32 = 4;
const LOCK_UN : i32 = 8;

pub enum LockType {
	Exclusive,
	Shared,
}
impl LockType {
	fn to_flock_flag(&self) -> i32 {
		match self {
			LockType::Exclusive => LOCK_EX,
			LockType::Shared => LOCK_SH,
		}
	}
}

pub enum BlockMode {
	Blocking,
	NonBlocking,
}
impl BlockMode {
	fn to_flock_flag(&self) -> i32 {
		match self {
			BlockMode::Blocking => 0,
			BlockMode::NonBlocking => LOCK_NB,
		}
	}
}

fn flogic(file: &File, flags: i32) -> Result<()> {
	#[cfg(unix)]
	// https://linux.die.net/man/2/flock
	let inner = move || {
		let ret = unsafe { flock(file.as_raw_fd(), flags) };
		if ret < 0 { Err(Error::last_os_error()) } else { Ok(()) }
	};

	#[cfg(windows)]
	// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
	let inner = move || {
		todo!();
		unsafe {
			let mut overlapped = std::mem::zeroed();
			let ret = winapi::um::fileapi::LockFileEx(file.as_raw_handle(), flags, 0, !0, !0, &mut overlapped);
			if ret == 0 { Err(Error::last_os_error()) } else { Ok(()) }
		}
	};

	inner()
}

/// Place a lock advisory on this file.
/// 
/// UNIX:
/// - Uses the `flock` syscall.
/// * `file` - A raw file descriptor will be extracted and passed to the flock syscall.
/// 
/// Windows:
/// - Uses the `LockFileEx` syscall (fileapi.h).
/// * `file` - A raw file handle will be extracted and passed to the LockFileEx syscall.
/// 
pub fn lock_file(file: &File, lock_type: LockType, block_mode: BlockMode) -> Result<()> {
	flogic(file, lock_type.to_flock_flag() | block_mode.to_flock_flag())
}

/// Remove a file lock advisory held by this process.
/// 
/// UNIX:
/// - Uses the `flock` syscall.
/// * `file` - A raw file descriptor will be extracted and passed to the flock syscall.
/// 
/// Windows:
/// - Uses the `UnlockFileEx` syscall (fileapi.h).
/// * `file` - A raw file handle will be extracted and passed to the UnlockFileEx syscall.
/// 
pub fn unlock_file(file: &File) -> Result<()> {
	flogic(file, LOCK_UN)
}

#[cfg(test)]
mod tests {
    use super::*;

	#[test]
	fn lock_unlock() {
		let lockfile_name = "lock_unlock.test.lock";
		let file = File::create(&lockfile_name).unwrap();
		lock_file(&file, LockType::Exclusive, BlockMode::NonBlocking).unwrap();
		unlock_file(&file).unwrap();
		std::fs::remove_file(lockfile_name).unwrap();
	}

	#[test]
	fn lock_works() {
		let lockfile_name = "lock_works.test.lock";
		let file = File::create(&lockfile_name).unwrap();

		std::process::Command::new("cargo")
			.arg("build")
			.current_dir("testing")
			.stdout(std::process::Stdio::null())
			.spawn()
			.expect("failed to spawn cargo to build test binary")
			.wait()
			.expect("failed to wait for the test binary to build");

		let mut child = std::process::Command::new("testing/target/debug/flack-test")
			.arg("lock")
			.arg(&lockfile_name)
			.spawn()
			.expect("failed to spawn the test binary");

		std::thread::sleep(std::time::Duration::from_millis(100));
		
		assert!(lock_file(&file, LockType::Exclusive, BlockMode::NonBlocking).is_err());

		child.kill().expect("failed to kill test binary");
		std::fs::remove_dir_all("testing/target").unwrap();
		std::fs::remove_file(lockfile_name).unwrap();
	}
}
