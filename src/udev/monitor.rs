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

use std::ffi::CString;
use std::io::Error;
use std::ptr;
use std::str::FromStr;

use udev::{
    device,
    libudev_c,
    util,
};
use udev::udev::Udev;
use udev::device::Device;

pub struct Monitor<'u> {
    udev: &'u Udev,
    monitor: libudev_c::udev_monitor
}

pub enum Action {
    Add,
    Remove,
    Change,
    Move,
    Online,
    Offline,
    Other(String)
}

pub struct Event {
    pub action: Action,
    pub seqnum: u64
}

#[doc(hidden)]
pub struct MonitorIterator<'m, 'u: 'm> {
    monitor: &'m Monitor<'u>
}

pub fn monitor(udev: &Udev, monitor: libudev_c::udev_monitor) -> Monitor {
    Monitor {
        udev: udev,
        monitor: monitor
    }
}

impl<'u> Monitor<'u> {
    /// Get the udev context.
    pub fn udev(&self) -> &Udev {
        self.udev
    }

    /// Filter by subsystem.
    ///
    /// Exclude devices that don't match the specified subsystem or a previously specified
    /// subsystem.
    pub fn filter_by_subsystem(self, subsystem: &str) -> Monitor<'u> {
        let cstr_subsystem = CString::new(subsystem).unwrap();
        util::handle_error(unsafe {
            libudev_c::udev_monitor_filter_add_match_subsystem_devtype(self.monitor,
                                                                       cstr_subsystem.as_ptr(),
                                                                       ptr::null())
        });
        self
    }
    /// Filter by subsystem/devtype combination.
    ///
    /// Exclude devices that don't match the specified subsystem/devtype combination or a
    /// previously specified subsystem/devtype combination (or any subsystem previously specified
    /// in a `filter_subsystem` invocation).
    pub fn filter_by_subsystem_devtype(self, subsystem: &str, devtype: &str) -> Monitor<'u> {
        let cstr_subsystem = CString::new(subsystem).unwrap();
        let cstr_devtype = CString::new(devtype).unwrap();
        util::handle_error(unsafe {
            libudev_c::udev_monitor_filter_add_match_subsystem_devtype(self.monitor,
                                                                       cstr_subsystem.as_ptr(),
                                                                       cstr_devtype.as_ptr())
        });
        self
    }
    /// Filter by tag.
    ///
    /// Exclude devices that don't match the specified tag or a previously specified tag.
    pub fn filter_by_tag(self, tag: &str) -> Monitor<'u> {
        let cstr_tag = CString::new(tag).unwrap();
        util::handle_error(unsafe {
            libudev_c::udev_monitor_filter_add_match_tag(self.monitor, cstr_tag.as_ptr())
        });
        self
    }

    /// Reset all filters on this monitor. No devices will be excluded.
    pub fn clear_filters(self) -> Monitor<'u> {
        util::handle_error(unsafe {
            libudev_c::udev_monitor_filter_remove(self.monitor)
        });
        self
    }

    /// Iterate over udev events.
    ///
    /// 1. The returned iterator will block on calls to next until their a device is available.
    /// 2. The returned iterator will never end (next will never return None).
    pub fn iter<'m>(&'m self) -> MonitorIterator<'m, 'u> {
        util::handle_error(unsafe {
            // Technically this mutates but we're single threaded anyways. Basically, having two
            // iterators existing at the same time won't cause any problems because next() can't be
            // called at the same time (single threaded).
            libudev_c::udev_monitor_enable_receiving(self.monitor)
        });
        MonitorIterator::<'m, 'u> {
            monitor: self
        }
    }
}

impl<'u> Drop for Monitor<'u> {
    fn drop(&mut self) {
        unsafe {
            libudev_c::udev_monitor_unref(self.monitor);
        }
    }
}

impl FromStr for Action {
    type Err = Error;
    fn from_str(s: &str) -> Result<Action, Self::Err> {
        use self::Action::*;

        match s {
            "add"       => Ok(Add),
            "remove"    => Ok(Remove),
            "change"    => Ok(Change),
            "move"      => Ok(Move),
            "online"    => Ok(Online),
            "offline"   => Ok(Offline),
            _           => Ok(Other(s.to_string())),
        }
    }
}

impl<'m, 'u> Iterator for MonitorIterator<'m, 'u> {

    type Item = (Event, Device<'u>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Ok(Some(dev)) = util::check_errno_mut(|| unsafe {
                libudev_c::udev_monitor_receive_device(self.monitor.monitor)
            }) {
                return Some((
                    Event {
                        action: Action::from_str(
                            util::c_to_str(
                                libudev_c::udev_device_get_action(dev))
                                .unwrap()).unwrap(),
                        seqnum: unsafe {
                            libudev_c::udev_device_get_seqnum(dev)
                        }
                    },
                    device::device(self.monitor.udev, dev)
                ));
            }
        }
    }

}

