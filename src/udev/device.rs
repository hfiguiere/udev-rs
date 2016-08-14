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

use libc::c_char;
use std::path::{Path,PathBuf};
use std::ptr;
use std::str::FromStr;
use std::io::{Error, ErrorKind};
use std::ffi::CString;
use std::fmt;
use time::Duration;

use libc::dev_t;

use udev::{
    libudev_c,
    util,
    iterator,
};
use udev::udev::Udev;
use udev::iterator::MappedIterator;

pub struct Device<'u> {
    udev: &'u Udev,
    dev: libudev_c::udev_device,
}

#[doc(hidden)]
pub type TagIterator<'p> = MappedIterator<'p, Device<'p>, &'p str>;
#[doc(hidden)]
pub type AttributeIterator<'p> = MappedIterator<'p, Device<'p>, &'p str>;
#[doc(hidden)]
pub type DevlinkIterator<'p> = MappedIterator<'p, Device<'p>, PathBuf>;
#[doc(hidden)]
pub type PropertyIterator<'p> = MappedIterator<'p, Device<'p>, (&'p str, Option<&'p str>)>;

pub type Devnum = dev_t;
pub enum Type {
    Char,
    Block
}

// Crate Private
pub fn device<'u>(udev: &'u Udev, dev: libudev_c::udev_device) -> Device<'u> {
    Device { udev: udev, dev: dev }
}

pub fn device_get_dev(device: &Device) -> libudev_c::udev_device {
    device.dev
}

impl<'u> Device<'u> {
    /// Get the udev context.
    pub fn udev(&self) -> &Udev {
        self.udev
    }

    /// Get the device's parent if one exists.
    pub fn parent(&self) -> Option<Device> {
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_device_ref(libudev_c::udev_device_get_parent(self.dev))
        }) {
            Ok(Some(dev)) => Some(device(self.udev, dev)),
            _ => None
        }
    }

    /// Get the first parent with the specified subsystem.
    pub fn parent_with_subsystem(&self, subsystem: &str) -> Option<Device> {
        let cstr_subsystem = CString::new(subsystem).unwrap();
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_device_ref(
                libudev_c::udev_device_get_parent_with_subsystem_devtype(self.dev,
                                                                         cstr_subsystem.as_ptr(),
                                                                         ptr::null()))
        }) {
            Ok(Some(dev)) => Some(device(self.udev, dev)),
            _ => None
        }
    }

    /// Get the first parent with the specified subsystem and devtype.
    pub fn parent_with_subsystem_devtype(&self, subsystem: &str, devtype: &str) -> Option<Device> {
        let cstr_subsystem = CString::new(subsystem).unwrap();
        let cstr_devtype = CString::new(devtype).unwrap();
        match util::check_errno_mut(|| unsafe {
            libudev_c::udev_device_ref(
                libudev_c::udev_device_get_parent_with_subsystem_devtype(
                    self.dev, cstr_subsystem.as_ptr(), cstr_devtype.as_ptr()))
        }) {
            Ok(Some(dev)) => Some(device(self.udev, dev)),
            _ => None
        }
    }

    /// Read a sysfs attribute.
    pub fn attribute<'s>(&'s self, attr: &str) -> Result<&'s str, Error> {
        let cstr_attr = CString::new(attr).unwrap();
        match util::check_errno(|| unsafe {
            libudev_c::udev_device_get_sysattr_value(self.dev, cstr_attr.as_ptr())
        }) {
            Ok(Some(val)) => Ok(util::c_to_str(val).unwrap()),
            Ok(None) => Err(Error::new(ErrorKind::NotFound, "")),
            Err(errno) => Err(Error::from_raw_os_error(errno)),
        }
    }

    /// Write a sysfs attribute.
    pub fn set_attribute(&self, attr: &str, value: &str) -> Result<(), Error> {
        let cstr_attr = CString::new(attr).unwrap();
        let cstr_value = CString::new(value).unwrap();
        match unsafe {
            libudev_c::udev_device_set_sysattr_value(self.dev,
                                                     cstr_attr.as_ptr(),
                                                     cstr_value.as_ptr())
        } {
            0           => Ok(()),
            n if n < 0  => Err(Error::from_raw_os_error(-n)),
            _           => panic!("udev returned an invalid error")
        }
    }

    /// Get the path to the device (minus `/sys`).
    pub fn devpath<'s>(&'s self) -> &'s str {
        util::c_to_str(libudev_c::udev_device_get_devpath(self.dev)).unwrap()
    }

    /// Get the full path to the device (including `/sys`).
    pub fn syspath<'s>(&'s self) -> PathBuf {
        PathBuf::from(util::c_to_str(libudev_c::udev_device_get_syspath(self.dev)).unwrap())
    }

    /// Get the device name.
    ///
    /// E.g. wlan0
    pub fn sysname<'s>(&'s self) -> &'s str {
        util::c_to_str(libudev_c::udev_device_get_sysname(self.dev)).unwrap()
    }

    /// Get the devices subsystem
    pub fn subsystem<'s>(&'s self) -> Option<&'s str> {
        util::c_to_str(libudev_c::udev_device_get_subsystem(self.dev))
    }

    /// Get the devices devtype
    pub fn devtype<'s>(&'s self) -> Option<&'s str> {
        util::c_to_str(libudev_c::udev_device_get_devtype(self.dev))
    }

    /// Get the devices sysnum.
    ///
    /// E.g. the X in ethX, wlanX, etc.
    pub fn sysnum(&self) -> Option<u64> {
        match util::c_to_str(libudev_c::udev_device_get_sysnum(self.dev)) {
            Some(n) => match u64::from_str(n) {
                Ok(i) => Some(i),
                Err(E) => None
            },
            None => None
        }
    }

    /// Get the device's devnum.
    pub fn devnum(&self) -> Option<Devnum> {
        match libudev_c::udev_device_get_devnum(self.dev) {
            0 => None,
            n => Some(n)
        }
    }

    /// Get the device's driver.
    pub fn driver(&self) -> Option<&str> {
        util::c_to_str(libudev_c::udev_device_get_driver(self.dev))
    }

    /// Get the device's devnode
    ///
    /// E.g. `/dev/sda`
    pub fn devnode(& self) -> Option<PathBuf> {
        util::c_to_str(libudev_c::udev_device_get_devnode(self.dev)).map(|path| PathBuf::from(path))
    }

    /// Iterate over the device's devlinks
    ///
    /// E.g. the symlinks in `/dev/disk/by-*/`
    pub fn iter_devlinks(&self) -> DevlinkIterator {
        iterator::iterator(self, libudev_c::udev_device_get_devlinks_list_entry(self.dev))
            .map(|(_, key, _)| PathBuf::from(key))
    }

    /// Iterate over the device's tags.
    pub fn iter_tags(&self) -> TagIterator {
        iterator::iterator(self, libudev_c::udev_device_get_tags_list_entry(self.dev))
            .map(|(_, key, _)| key)
    }

    /// Iterate over the device's properties.
    pub fn iter_properties(&self) -> PropertyIterator {
        iterator::iterator(self, libudev_c::udev_device_get_properties_list_entry(self.dev))
            .map(|(_, key, value)| (key, value))
    }

    /// Iterate over the device's sysfs attribute names
    pub fn iter_attributes(& self) -> AttributeIterator {
        iterator::iterator(self, libudev_c::udev_device_get_sysattr_list_entry(self.dev))
            .map(|(_, key, _)| key)
    }

    /// Get the time since the device was initialized by udev.
    pub fn time_since_initialized(&self) -> Option<Duration> {
        let usec = unsafe { libudev_c::udev_device_get_usec_since_initialized(self.dev) };
        if usec == 0 {
            None
        } else {
            // Note: I don't support machines that are online for over 292,471 years. Sorry.
            Some(Duration::microseconds(usec as i64))
        }
    }

    /// Determine if the device has been initialized.
    pub fn is_initialized(&self) -> bool {
        unsafe { libudev_c::udev_device_get_is_initialized(self.dev) != 0 }
    }

    /// Check whether the device is tagged with a given tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        let cstr_tag = CString::new(tag).unwrap();
        unsafe {
            libudev_c::udev_device_has_tag(self.dev, cstr_tag.as_ptr()) != 0
        }
    }
}

impl<'u> Drop for Device<'u> {
    fn drop(&mut self) {
        unsafe { libudev_c::udev_device_unref(self.dev) };
    }
}

impl<'u> fmt::Debug for Device<'u> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.syspath().to_str().unwrap())
    }
}

impl Type {
    pub fn to_char(self) -> i8 {
        match self {
            Type::Char => 'c' as i8,
            Type::Block => 'b' as i8
        }
    }
}
