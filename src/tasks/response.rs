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
//use std::collections::HashMap;
use crate::connection::command::CommandResult;
use crate::tasks::logic::{PreLogicEvaluated,PostLogicEvaluated};
use crate::tasks::fields::Field;
use std::vec::Vec;

// task responses are returns from module calls - they are not
// created directly but by helper functions in handle.rs, see
// the various modules for examples/usage

#[derive(Debug,PartialEq)]
pub enum TaskStatus {
    IsCreated,
    IsRemoved,
    IsModified,
    IsExecuted,
    IsPassive,
    IsMatched,
    IsSkipped,
    NeedsCreation,
    NeedsRemoval,
    NeedsModification,
    NeedsExecution,
    NeedsPassive,
    Failed
}

#[derive(Debug)]
pub struct TaskResponse {
    pub status: TaskStatus,
    pub changes: Vec<Field>,
    pub msg: Option<String>,
    pub command_result: Arc<Option<CommandResult>>,
    #[allow(dead_code)] // FIXME: remove if truly not needed
    pub with: Arc<Option<PreLogicEvaluated>>,
    #[allow(dead_code)] // FIXME: remove if truly not needed
    pub and: Arc<Option<PostLogicEvaluated>>
}

//impl TaskResponse {
//}
