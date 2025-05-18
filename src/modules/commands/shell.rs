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
use crate::connection::command::cmd_info;
use serde::Deserialize;
use std::sync::{Arc,RwLock};
use crate::inventory::hosts::Host;

const MODULE: &str = "Shell";

#[derive(Deserialize,Debug)]
#[serde(deny_unknown_fields)]
pub struct ShellTask {
    pub name: Option<String>,
    pub cmd: String,
    pub save: Option<String>, 
    pub failed_when: Option<String>, 
    pub changed_when: Option<String>, 
    #[serde(rename = "unsafe")]
    pub unsafe_: Option<String>, /* FIXME: can use r#unsafe instead */
    pub with: Option<PreLogicInput>,
    pub and: Option<PostLogicInput>,
}
struct ShellAction {
    pub cmd: String,
    pub save: Option<String>, 
    pub failed_when: Option<String>,
    pub changed_when: Option<String>,
    pub unsafe_: bool,
}


impl IsTask for ShellTask {

    fn get_module(&self) -> String { String::from(MODULE) }
    fn get_name(&self) -> Option<String> { self.name.clone() }
    fn get_with(&self) -> Option<PreLogicInput> { self.with.clone() }

    fn evaluate(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>, tm: TemplateMode) -> Result<EvaluatedTask, Arc<TaskResponse>> {
        Ok(
            EvaluatedTask {
                action: Arc::new(ShellAction {
                    unsafe_:  {
                        if !self.cmd.contains("{{") {
                            // allow all the fancy shell characters unless variables are used, in which case
                            // do a bit of extra filtering unless users turn it off.
                            true
                        } else {
                            handle.template.boolean_option_default_false(request, tm, &String::from("unsafe"), &self.unsafe_)?
                        }
                    },
                    cmd:  handle.template.string_unsafe_for_shell(request, tm, &String::from("cmd"), &self.cmd)?,
                    save: handle.template.string_option_no_spaces(request, tm, &String::from("save"), &self.save)?,
                    failed_when: handle.template.string_option_unsafe_for_shell(request, tm, &String::from("failed_when"), &self.failed_when)?,
                    changed_when: handle.template.string_option_unsafe_for_shell(request, tm, &String::from("changed_when"), &self.changed_when)?,

                }),
                with: Arc::new(PreLogicInput::template(handle, request, tm, &self.with)?),
                and: Arc::new(PostLogicInput::template(handle, request, tm, &self.and)?),
            }
        )
    }

}

impl IsAction for ShellAction {
    
    fn dispatch(&self, handle: &Arc<TaskHandle>, request: &Arc<TaskRequest>) -> Result<Arc<TaskResponse>, Arc<TaskResponse>> {
    
        match request.request_type {

            TaskRequestType::Query => {
                Ok(handle.response.needs_execution(request))
            },

            TaskRequestType::Execute => {
                let task_result: Arc<TaskResponse> = if self.unsafe_ {
                    handle.remote.run_unsafe(request, &self.cmd.clone(), CheckRc::Unchecked)?
                } else {
                    handle.remote.run(request, &self.cmd.clone(), CheckRc::Unchecked)?
                };
                let (rc, out) = cmd_info(&task_result);
                let map_data = build_results_map(rc, &out);

                let should_fail = match self.failed_when.is_none() {
                    true => !matches!(rc, 0),
                    false => {
                        let condition = self.failed_when.as_ref().unwrap();
                        handle.template.test_condition_with_extra_data(request, TemplateMode::Strict, condition, &handle.host, map_data.clone())?
                    }
                };

                let should_mark_changed = match self.changed_when.is_none() {
                    true => true,
                    false => {
                        let condition = self.changed_when.as_ref().unwrap();
                        handle.template.test_condition_with_extra_data(request, TemplateMode::Strict, condition, &handle.host, map_data.clone())?
                    }
                };

                if self.save.is_some() {
                    save_results(&handle.host, self.save.as_ref().unwrap(), map_data);
                }

                match should_fail {
                    true => Err(handle.response.command_failed(request, &Arc::clone(&task_result.command_result))),
                    false => match should_mark_changed {
                        true => Ok(task_result),
                        false => Ok(handle.response.is_passive(request))
                    }
                }

            },
    
            _ => { Err(handle.response.not_supported(request))}
    
        }
    }

}

fn build_results_map(rc: i32, out: &str) -> serde_yaml::Mapping {
    let mut result = serde_yaml::Mapping::new();
    let num : serde_yaml::Value = serde_yaml::from_str(&format!("{}", rc)).unwrap();
    result.insert(serde_yaml::Value::String(String::from("rc")), num);
    //result.insert(serde_yaml::Value::String(String::from("rc")),  serde_yaml::Value::String(format!("{}", rc)));

    result.insert(serde_yaml::Value::String(String::from("out")), serde_yaml::Value::String(out.to_owned()));
    result
}

fn save_results(host: &Arc<RwLock<Host>>, key: &str, map_data: serde_yaml::Mapping) {
    let mut result = serde_yaml::Mapping::new();
    result.insert(serde_yaml::Value::String(key.to_owned()), serde_yaml::Value::Mapping(map_data.clone()));
    host.write().unwrap().update_variables(result);
}
