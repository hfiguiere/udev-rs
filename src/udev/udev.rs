// This file is part of udev-rs.
// 
// Copyright 2014 Steven Allen <steven@stebalien.com>
// 
// udev-rs is free software; you can redistribute it and/or modify it
// under the terms of the GNU Lesser General Public License as published by
// the Free Software Foundation; either version 2.1 of the License, or
// (at your option) any later version.
// 
// udev-rs is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Lesser General Public License for more details.
// 
// You should have received a copy of the GNU Lesser General Public License
// along with udev-rs; If not, see <http://www.gnu.org/licenses/>.

use std::cell::UnsafeCell;
use std::ffi::CString;
use std::io::Error;
use std::path::Path;

use libc::{
    fcntl,
    O_NONBLOCK,
    F_SETFL,
    F_GETFL,
    ENOMEM,
    EINVAL,
};
use alloc::oom;

use udev::{
    device,
    util,
    hwdb,
    monitor,
    enumerator,
    libudev_c,
};
use udev::device::{
    Device,
};
use udev::hwdb::Hwdb;
use udev::monitor::Monitor;
use udev::enumerator::Enumerator;

pub struct Udev {
    // Not thread safe. As all children will hold a reference, this makes everything safe.
    udev: UnsafeCell<libudev_c::udev>
}

impl Udev {
    /// Create a new udev handle.
    pub fn new() -> Udev {
        let udev = unsafe { libudev_c::udev_new() };
        // I don't care about errno. NULL == oom.
        if udev.is_null() {
            oom();
        }
        Udev { udev: UnsafeCell::new(udev) }
    }

    fn create_monitor(&self, name: &str) -> Result<Monitor, Error>  {
        let cstr_name = CString::new(name).unwrap();
        let monitor = match util::check_errno_mut(|| unsafe {
            libudev_c::udev_monitor_new_from_netlink(self.udev.into_inner(), cstr_name.as_ptr())
        }) {
            Ok(Some(monitor))       => monitor,
            Err(EINVAL) | Ok(None)  => panic!("BUG"),
            Err(e)                  => return Err(Error::from_raw_os_error(e))
        };
        let fd = unsafe {
            libudev_c::udev_monitor_get_fd(monitor)
        };

        let old_val = unsafe { fcntl(fd, F_GETFL) };
        if old_val == -1 || unsafe { fcntl(fd, F_SETFL, old_val & !O_NONBLOCK) == -1 } {
            return match util::get_errno() {
                ENOMEM | EINVAL => panic!("BUG"),
                e => Err(Error::from_raw_os_error(e))
            }
        }

        Ok(monitor::monitor(self, monitor))
    }

    /// Monitor udev events.
    ///
    /// # Error
    ///
    /// This will return an error if you're running in an environment without access to netlink.
    pub fn monitor(&self) -> Result<Monitor, Error> {
        self.create_monitor("udev")
    }

    /// Monitor kernel events.
    ///
    /// # Error
    ///
    /// This method will return an error if you're running in an environment without access to
    /// netlink.
    ///
    /// # Safety Notes
    ///
    /// This method is marked unsafe due to the following warning found in libudev:
    ///
    /// > Applications should usually not connect directly to the
    /// > "kernel" events, because the devices might not be useable
    /// > at that time, before udev has configured them, and created
    /// > device nodes. Accessing devices at the same time as udev,
    /// > might result in unpredictable behavior. The "udev" events
    /// > are sent out after udev has finished its event processing,
    /// > all rules have been processed, and needed device nodes are
    /// > created.
    pub unsafe fn monitor_kernel(&self) -> Result<Monitor, Error> {
        self.create_monitor("kernel")
    }

    /// Create a new hardware database handle.
    ///
    /// # Error
    ///
    /// On error, this method will return either Err(errno) or Err(0). Err(errno) indicates a
    /// problem reading the hardware database and Err(0) indicates that the hardware database is
    /// corrupt.
    pub fn hwdb(&self) -> Result<Hwdb, i32> {
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_hwdb_new(self.udev.into_inner())
        }) {
            Ok(Some(hwdb))  => Ok(hwdb::hwdb(self, hwdb)),
            Ok(None)        => Err(0i32),
            Err(EINVAL)     => panic!("BUG"),
            Err(e)          => Err(e)
        }
    }

    /// Lookup a device by sys path.
    pub fn device(&self, path: &Path) -> Option<Device> {
        let cstr_path = CString::new(path.to_str().unwrap()).unwrap();
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_device_new_from_syspath(self.udev.into_inner(), cstr_path.as_ptr())
        }) {
            Ok(Some(dev)) => Some(device::device(self, dev)),
            _ => None
        }
    }

    /// Lookup a device by device type and device number.
    pub fn device_from_devnum(&self, ty: device::Type, devnum: device::Devnum) -> Option<Device> {
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_device_new_from_devnum(self.udev.into_inner(), ty.to_char(), devnum)
        }) {
            Ok(Some(dev)) => Some(device::device(self, dev)),
            _ => None
        }
    }

    /// Lookup a device by subsystem and sysname
    pub fn device_from_subsystem_sysname(&self, subsystem: &str, sysname: &str) -> Option<Device> {
        let cstr_sysname = CString::new(sysname).unwrap();
        let cstr_subsystem = CString::new(subsystem).unwrap();
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_device_new_from_subsystem_sysname(self.udev.into_inner(),
                                                              cstr_subsystem.as_ptr(),
                                                              cstr_sysname.as_ptr())
        }) {
            Ok(Some(dev)) => Some(device::device(self, dev)),
            _ => None
        }
    }

    /// Create a device enumerator.
    pub fn enumerator(&self) -> Enumerator {
        enumerator::enumerator(
            self, util::check_errno_mut(|| {
                libudev_c::udev_enumerate_new(self.udev.into_inner())
            }).unwrap().unwrap())
    }
}

impl Drop for Udev {
    fn drop(&mut self) {
        unsafe { libudev_c::udev_unref(self.udev.into_inner()) };
    }
}
