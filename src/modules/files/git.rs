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

use crate::tasks::*;
use crate::handle::handle::TaskHandle;
use crate::tasks::fields::Field;
use serde::Deserialize;
use std::sync::Arc;
use std::vec::Vec;
use crate::tasks::files::Recurse;
use std::collections::HashMap;

const MODULE: &str = "git";

#[derive(Deserialize,Debug)]
#[serde(deny_unknown_fields)]
pub struct GitTask {
    pub name: Option<String>,
    pub repo: String,
    pub path: String,
    pub branch: Option<String>,
    pub ssh_options: Option<HashMap<String,String>>,
    pub accept_keys: Option<String>,
    pub update: Option<String>,
    pub attributes: Option<FileAttributesInput>,
    pub with: Option<PreLogicInput>,
    pub and: Option<PostLogicInput>
}

struct GitAction {
    pub repo: String,
    pub path: String,
    pub branch: String,
    pub ssh_options: Vec<String>,
    pub accept_keys: bool,
    pub update: bool,
    pub attributes: Option<FileAttributesEvaluated>,
}

impl IsTask for GitTask {

    fn get_module(&self) -> String { String::from(MODULE) }
    fn get_name(&self) -> Option<String> { self.name.clone() }
    fn get_with(&self) -> Option<PreLogicInput> { self.with.clone() }

    fn evaluate(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>, tm: TemplateMode) -> Result<EvaluatedTask, Arc<TaskResponse>> {
        Ok(
            EvaluatedTask {
                action: Arc::new(GitAction {
                    repo:         handle.template.string(request, tm, &String::from("repo"), &self.repo)?,
                    path:         handle.template.path(request, tm, &String::from("path"), &self.path)?,
                    branch:       handle.template.string_option_default(request, tm, &String::from("branch"), &self.branch, &String::from("main"))?,
                    accept_keys:  handle.template.boolean_option_default_true(request, tm, &String::from("accept_keys"), &self.accept_keys)?,
                    update:       handle.template.boolean_option_default_true(request, tm, &String::from("update"), &self.update)?,
                    attributes:   FileAttributesInput::template(handle, request, tm, &self.attributes)?,
                    ssh_options:  {
                        let mut options : Vec<String> = Vec::new();
                        if let Some(input_options) = &self.ssh_options {
                            for (k,v) in input_options.iter() {
                                options.push(format!("-o {}={}", k, v))
                            }
                        };
                        options.push(String::from("-o BatchMode=Yes"));
                        options
                    }
                }),
                with: Arc::new(PreLogicInput::template(handle, request, tm, &self.with)?),
                and: Arc::new(PostLogicInput::template(handle, request, tm, &self.and)?),
            }
        )
    }

}

impl IsAction for GitAction {

    fn dispatch(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<Arc<TaskResponse>, Arc<TaskResponse>> {
    
        match request.request_type {

            TaskRequestType::Query => {

                let mut changes : Vec<Field> = Vec::new();
                // see if the remote directory exists
                let remote_mode = handle.remote.query_common_file_attributes(request, &self.path, &self.attributes, &mut changes, Recurse::Yes)?;                 

                match remote_mode {
                    // the directory does not exist, need to make everything happen
                    None => Ok(handle.response.needs_creation(request)),

                    // the directory does exist, but the .git directory might not, or it might need to change versions/branches
                    // so more checking needed...
                    _ => {
                        
                        let git_path = match self.path.ends_with("/") {
                            // could have used pathbuf, but ... anyway ...
                            true => format!("{}{}", self.path, String::from(".git")),
                            false => format!("{}/{}", self.path, String::from(".git")),
                        };

                        match handle.remote.get_mode(request, &git_path)? {

                            // the repo does not exist, so do everything
                            None => Ok(handle.response.needs_creation(request)),

                            // the repo does exist, see what needs to change depending on parameters
                            // minor FIXME: this module does not currently deal with repo URLs changing
                            // when a git directory has already been checked out at a given location
                            _ => {
                                let local_version = self.get_local_version(handle, request)?;
                                if local_version.is_none() {
                                    changes.push(Field::Version);
                                }
                                else {
                                    let remote_version = self.get_remote_version(handle, request)?;
                                    let local_branch = self.get_local_branch(handle, request)?;
                                    if self.update && (! remote_version.eq(&local_version.unwrap())) {
                                        changes.push(Field::Version);
                                    }
                                    if ! local_branch.eq(&self.branch) {
                                        changes.push(Field::Branch);
                                    }
                                }

                                if !changes.is_empty() {
                                    Ok(handle.response.needs_modification(request, &changes))
                                } else {
                                    Ok(handle.response.is_matched(request))

                                }
                            }
                        }
                    }
                }
            }
                
            TaskRequestType::Create => {
                handle.remote.create_directory(request, &self.path)?;
                handle.remote.process_all_common_file_attributes(request, &self.path, &self.attributes, Recurse::Yes)?;
                self.clone(handle, request)?;
                self.switch_branch(handle, request)?;                           
                Ok(handle.response.is_created(request))
            },

            TaskRequestType::Modify => {

                handle.remote.process_common_file_attributes(request, &self.path, &self.attributes, &request.changes, Recurse::Yes)?;
                if request.changes.contains(&Field::Branch) || request.changes.contains(&Field::Version) {
                    self.pull(handle,request)?;
                }
                if request.changes.contains(&Field::Branch) {
                    self.switch_branch(handle, request)?;
                }
                Ok(handle.response.is_modified(request, request.changes.clone()))
            },

            // no passive or execute leg
            _ => { Err(handle.response.not_supported(request))}

        
        }
    }
}

impl GitAction {

    // BOOKMARK: fleshing this all out... 

    fn is_ssh_repo(&self) -> bool {
        self.repo.contains("@") || self.repo.contains("ssh://")
    }

    fn get_ssh_options_string(&self) -> String {
        let options = self.ssh_options.join(" ");
        if self.path.starts_with("http") {
            // http or https:// passwords are intentionally not supported, use a key instead, see docs
            String::from("GIT_TERMINAL_PROMPT=0")
        }
        else {
            let accept_keys = match self.accept_keys {
                true  => String::from(" -o StrictHostKeyChecking=accept-new"),
                false => String::from("")
            };
            format!("GIT_SSH_COMMAND=\"ssh {}{}\" GIT_TERMINAL_PROMPT=0", options, accept_keys)
        }
    }

    fn get_local_version(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<Option<String>, Arc<TaskResponse>> {
        let cmd = format!("git -C {} rev-parse HEAD", self.path);
        let result = handle.remote.run_unsafe(request, &cmd, CheckRc::Unchecked)?;
        let (rc, out) = cmd_info(&result);
        if rc == 0 {
            Ok(Some(out.replace("\n","")))
        } else {
            Ok(None)
        }
    }

    fn get_remote_version(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<String, Arc<TaskResponse>> {
        let ssh_options = self.get_ssh_options_string();
        let cmd = format!("{} git ls-remote {} | head -n 1 | cut -f 1", ssh_options, self.repo);
        let result = match self.is_ssh_repo() {
            true  => handle.remote.run_forwardable(request, &cmd, CheckRc::Checked)?,
            false => handle.remote.run_unsafe(request, &cmd, CheckRc::Checked)?
        };
        let (_rc, out) = cmd_info(&result);
        Ok(out)
    }
    

    fn pull(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<(), Arc<TaskResponse>> {
        let ssh_options = self.get_ssh_options_string();
        let cmd = format!("{} git -C {} pull", ssh_options, self.path);
        match self.is_ssh_repo() {
            true  => handle.remote.run_forwardable(request, &cmd, CheckRc::Checked)?,
            false => handle.remote.run_unsafe(request, &cmd, CheckRc::Checked)?
        };
        Ok(())
    }

    fn get_local_branch(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<String, Arc<TaskResponse>> {
        let cmd = format!("git -C {} rev-parse --abbrev-ref HEAD", self.path);
        let result = handle.remote.run_unsafe(request, &cmd, CheckRc::Checked)?;
        let (_rc, out) = cmd_info(&result);
        Ok(out)
    }

    fn clone(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<(),Arc<TaskResponse>> {
        let ssh_options = self.get_ssh_options_string();
        handle.remote.create_directory(request, &self.path)?;
        let cmd = format!("{} git clone {} {}", ssh_options, self.repo, self.path);
        match self.is_ssh_repo() {
            true =>  handle.remote.run_forwardable(request, &cmd, CheckRc::Checked)?,
            false => handle.remote.run_unsafe(request, &cmd, CheckRc::Checked)?
        };
        Ok(())
    }

    fn switch_branch(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<(), Arc<TaskResponse>> {
        let cmd = format!("git -C {} switch {}", self.path, self.branch);
        handle.remote.run_unsafe(request, &cmd, CheckRc::Checked)?;
        Ok(())
    }

}
// TODO: agent forwarding flag used by SSH connections
// + make stuff work
// + testing ssh and http repos without passwords
// branch changes 
// etc
