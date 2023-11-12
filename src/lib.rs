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

// const LOCK_SH : i32 = 1; // shared lock
const LOCK_EX : i32 = 2; // exclusive lock
const LOCK_NB : i32 = 4; // don't block when locking
const LOCK_UN : i32 = 8; // unlock

pub fn lock_file(file: &File) -> Result<()> {
	#[cfg(unix)]
	// https://linux.die.net/man/2/flock
	let inner = move || {
		let ret = unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) };
		if ret < 0 { Err(Error::last_os_error()) } else { Ok(()) }
	};

	#[cfg(windows)]
	// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
	let inner = move || {
		unsafe {
			let mut overlapped = std::mem::zeroed();
			let ret = winapi::um::fileapi::LockFileEx(file.as_raw_handle(), flags, 0, !0, !0, &mut overlapped);
			if ret == 0 { Err(Error::last_os_error()) } else { Ok(()) }
		}
	};

	inner()
}

pub fn unlock_file(file: &File) -> Result<()> {
	#[cfg(unix)]
	// https://linux.die.net/man/2/flock
	let inner = move || {
		let ret = unsafe { flock(file.as_raw_fd(), LOCK_UN) };
		if ret < 0 { Err(Error::last_os_error()) } else { Ok(()) }
	};

	#[cfg(windows)]
	// https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-lockfileex
	let inner = move || {
		unsafe {
			let mut overlapped = std::mem::zeroed();
			let ret = winapi::um::fileapi::UnlockFileEx(file.as_raw_handle(), flags, 0, !0, !0, &mut overlapped);
			if ret == 0 { Err(Error::last_os_error()) } else { Ok(()) }
		}
	};

	inner()
}

#[cfg(test)]
mod tests {
    use super::*;

	static TESTING_FLOCK_PATH: &str = "testing/flock";

	#[test]
	fn lock_unlock() {
		let lockfile_name = "lock_unlock.test.lock";
		let file = File::create(&lockfile_name).unwrap();
		lock_file(&file).unwrap();
		unlock_file(&file).unwrap();
		std::fs::remove_file(lockfile_name).unwrap();
	}

	#[test]
	fn lock_works() {
		let lockfile_name = "lock_works.test.lock";
		let file = File::create(&lockfile_name).unwrap();

		std::process::Command::new("cargo")
			.arg("build")
			.current_dir(TESTING_FLOCK_PATH)
			.stdout(std::process::Stdio::null())
			.spawn()
			.expect("failed to spawn cargo to build flock")
			.wait()
			.expect("failed to wait for flock to build");

		let mut child = std::process::Command::new(TESTING_FLOCK_PATH.to_owned() + "/target/debug/flock")
			.arg("lock")
			.arg(&lockfile_name)
			.spawn()
			.expect("failed to spawn flock");

		std::thread::sleep(std::time::Duration::from_millis(100));
		
		assert!(lock_file(&file).is_err());

		child.kill().expect("failed to kill flock");
		std::fs::remove_dir_all(TESTING_FLOCK_PATH.to_owned() + "/target").unwrap();
		std::fs::remove_file(lockfile_name).unwrap();
	}
}
