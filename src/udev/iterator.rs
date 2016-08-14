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

use std::iter;

use udev::{
    libudev_c,
    util
};

// TODO: I could do all of this functionally (map/filter style) but that would make the return
// types a total mess. Therefore, I don't.
//
// When rust finally adds that feature, I can get rid of most of this file...

// Create private.
pub struct UdevIterator<'p, T: 'p + ?Sized> {
    parent: &'p T,
    entry: libudev_c::udev_list_entry
}

impl<'p, T: ?Sized> Iterator for UdevIterator<'p, T> {

    type Item = (&'p T, &'p str, Option<&'p str>);

    fn next(&mut self) -> Option<Self::Item> {
        if self.entry.is_null() {
            None
        } else {
            let ret = Some((
                self.parent,
                util::c_to_str(
                    libudev_c::udev_list_entry_get_name(self.entry)).unwrap(),
                util::c_to_str(
                    libudev_c::udev_list_entry_get_value(self.entry))
            ));
            self.entry = unsafe { libudev_c::udev_list_entry_get_next(self.entry) };
            ret
        }
    }
}

pub fn iterator<'a, T: 'a + ?Sized>(parent: &'a T, entry: libudev_c::udev_list_entry) -> UdevIterator<'a, T> {
    UdevIterator {
        parent: parent,
        entry: entry,
    }
}

pub type MappedIterator<'p, P: 'p, O> = iter::Map<O, UdevIterator<'p, P>>;
pub type FilterMappedIterator<'p, P: 'p, O> = iter::FilterMap<O, UdevIterator<'p, P>>;

