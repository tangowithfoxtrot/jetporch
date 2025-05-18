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

use crate::tasks::request::TaskRequest;
use crate::tasks::response::TaskResponse;
use crate::handle::response::Response;
use std::sync::Arc;
use std::marker::{Send,Sync};
use std::path::Path;
use crate::connection::command::Forward;

// the connection trait that serves as the base for SshConnection, LocalConnection, and NoConnection

pub trait Connection : Send + Sync {

    fn connect(&mut self) -> Result<(),String>;  

    // FIXME: add error return objects
    
    fn write_data(&self, response: &Arc<Response>, request: &Arc<TaskRequest>, data: &str, remote_path: &str) -> Result<(),Arc<TaskResponse>>;

    fn copy_file(&self, response: &Arc<Response>, request: &Arc<TaskRequest>, src: &Path, dest: &str) -> Result<(), Arc<TaskResponse>>;

    fn whoami(&self) -> Result<String,String>;

    fn run_command(&self, response: &Arc<Response>, request: &Arc<TaskRequest>, cmd: &str, forward: Forward) -> Result<Arc<TaskResponse>,Arc<TaskResponse>>;

}
