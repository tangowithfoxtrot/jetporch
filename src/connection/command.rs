// Jetporch
// Copyright (C) 2023 - Michael DeHaan <michael@michaeldehaan.net> + contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// at your option) any later version.
// 
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
// 
// You should have received a copy of the GNU General Public License
// long with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::sync::Arc;
use crate::tasks::response::TaskResponse;

// details useful for working with commands
// not much here, see handle/remote.rs for more

#[derive(Clone,Debug)]
pub struct CommandResult {
    pub cmd: String,
    pub out: String,
    pub rc: i32
}

#[derive(Debug,Copy,Clone,PartialEq)]
pub enum Forward {
    Yes,
    No
}

pub fn cmd_info(info: &Arc<TaskResponse>) -> (i32, String) {
    assert!(info.command_result.is_some(), "called cmd_info on a response that is not a command result");
    let result = info.command_result.as_ref().as_ref().unwrap();
    (result.rc, result.out.clone())
}
