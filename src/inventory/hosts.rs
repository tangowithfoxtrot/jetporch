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

use std::collections::HashMap;
use crate::util::yaml::blend_variables;
use std::sync::Arc;
use crate::inventory::groups::Group;
use std::sync::RwLock;
use std::collections::HashSet;
use serde_yaml;

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum HostOSType {
    Linux,
    MacOS,
}

#[derive(Clone,Copy,Debug)]
pub enum PackagePreference {
    // other package systems are supported but no other OSes are 'fuzzy' between distro families (yet)
    // so we don't need to specify them here (yet)
    Dnf,
    Yum,
}

pub struct Host {
    pub name               : String,
    pub groups             : HashMap<String, Arc<RwLock<Group>>>,
    pub variables          : serde_yaml::Mapping,
    pub os_type            : Option<HostOSType>,
    checksum_cache         : HashMap<String,String>,
    checksum_cache_task_id : usize,
    facts                  : serde_yaml::Value,
    pub package_preference : Option<PackagePreference>,
    notified_handlers      : HashMap<usize, HashSet<String>>
}

impl Host {

    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            variables : serde_yaml::Mapping::new(),
            groups: HashMap::new(),
            os_type: None,
            checksum_cache: HashMap::new(),
            checksum_cache_task_id: 0,
            facts: serde_yaml::Value::from(serde_yaml::Mapping::new()),
            notified_handlers: HashMap::new(),
            package_preference: None
        }
    }

    pub fn notify(&mut self, play_number: usize, signal: &str) {
        self.notified_handlers.entry(play_number).or_default();
        let entry = self.notified_handlers.get_mut(&play_number).unwrap();
        entry.insert(signal.to_owned());
    }

    pub fn is_notified(&self, play_number: usize, signal: &str) -> bool {
        let entry = self.notified_handlers.get(&play_number);
        if let Some(e) = entry  {
            e.contains(&signal.to_owned())
        } else {
            false
        }
    }

    pub fn set_checksum_cache(&mut self, path: &str, checksum: &str) {
        self.checksum_cache.insert(path.to_owned(), checksum.to_owned());
    }

    pub fn get_checksum_cache(&mut self, task_id: usize, path: &String) -> Option<String> {
        if task_id > self.checksum_cache_task_id {
            self.checksum_cache_task_id = task_id;
            self.checksum_cache.clear();
        }
        if self.checksum_cache.contains_key(path) {
            let result = self.checksum_cache.get(path).unwrap();
            Some(result.clone())
        }
        else {
            None
        }
    }

    // used by connection class on initial connect
    pub fn set_os_info(&mut self, uname_output: &String) -> Result<(),String> {
        if uname_output.starts_with("Linux")   { self.os_type = Some(HostOSType::Linux)   }
        else if uname_output.starts_with("Darwin")  { self.os_type = Some(HostOSType::MacOS)   }
        else {
            return Err(format!("OS Type could not be detected from uname -a: {}", uname_output));
        }
        Ok(())
    }

    // ==============================================================================================================
    // PUBLIC API - most code can use this
    // ==============================================================================================================
  
    pub fn get_groups(&self) -> HashMap<String, Arc<RwLock<Group>>> {
        let mut results : HashMap<String, Arc<RwLock<Group>>> = HashMap::new();
        for (k,v) in self.groups.iter() {
            results.insert(k.clone(), Arc::clone(v));
        }
        results
    }

    pub fn has_group(&self, group_name: &String) -> bool {
        for (k,_v) in self.groups.iter() {
            if k == group_name {
                return true;
            }
        }
        false
    }

    // get_ancestor_groups(&self, depth_limit: usize) -> HashMap<String, Arc<RwLock<Group>>>

    pub fn has_ancestor_group(&self, group_name: &String) -> bool {
        for (k,v) in self.groups.iter() {
            if k == group_name {
                return true;
            }
            for (k2,_v2) in v.read().unwrap().get_ancestor_groups(10) {
                if k2 == group_name.clone() {
                    return true;
                }
            }
        }
        false
    }

    pub fn get_group_names(&self) -> Vec<String> {
        self.get_groups().keys().cloned().collect()
    }

    pub fn add_group(&mut self, name: &str, group: Arc<RwLock<Group>>) {
        self.groups.insert(name.to_owned(), Arc::clone(&group));
    }

    pub fn get_ancestor_groups(&self, depth_limit: usize) -> HashMap<String, Arc<RwLock<Group>>> {

        let mut results : HashMap<String, Arc<RwLock<Group>>> = HashMap::new();
        for (k,v) in self.get_groups().into_iter() {
            results.insert(k, Arc::clone(&v));
            for (k2,v2) in v.read().expect("group read").get_ancestor_groups(depth_limit).into_iter() { 
                results.insert(k2, Arc::clone(&v2)); 
            }
        }
        results
    }

    pub fn get_ancestor_group_names(&self) -> Vec<String> {
        self.get_ancestor_groups(20usize).keys().cloned().collect()
    }

    pub fn get_variables(&self) -> serde_yaml::Mapping {
        self.variables.clone()
    }

    pub fn set_variables(&mut self, variables: serde_yaml::Mapping) {
        self.variables = variables.clone();
    }

    pub fn update_variables(&mut self, mapping: serde_yaml::Mapping) {
        for (k,v) in mapping.iter() {
            self.variables.insert(k.clone(),v.clone());
        }
    }

    pub fn get_blended_variables(&self) -> serde_yaml::Mapping {
        let mut blended : serde_yaml::Value = serde_yaml::Value::from(serde_yaml::Mapping::new());
        let ancestors = self.get_ancestor_groups(20);
        for (_k,v) in ancestors.iter() {
            let theirs : serde_yaml::Value = serde_yaml::Value::from(v.read().unwrap().get_variables());
            blend_variables(&mut blended, theirs);
        }
        let mine = serde_yaml::Value::from(self.get_variables());
        blend_variables(&mut blended, mine);
        blend_variables(&mut blended, self.facts.clone());
        match blended {
            serde_yaml::Value::Mapping(x) => x,
            _ => panic!("get_blended_variables produced a non-mapping (1)")
        }
    }

    pub fn update_facts(&mut self, mapping: &Arc<RwLock<serde_yaml::Mapping>>) {
        let map = mapping.read().unwrap().clone();
        blend_variables(&mut self.facts, serde_yaml::Value::Mapping(map));
    }

    pub fn update_facts2(&mut self, mapping: serde_yaml::Mapping) {
        blend_variables(&mut self.facts, serde_yaml::Value::Mapping(mapping));
    }

    pub fn get_variables_yaml(&self) -> Result<String, String> {
        let result = serde_yaml::to_string(&self.get_variables());
        match result {
            Ok(x) => Ok(x),
            Err(_y) => Err(String::from("error loading variables"))
        }
    }

    pub fn get_blended_variables_yaml(&self) -> Result<String,String> {
        let result = serde_yaml::to_string(&self.get_blended_variables());
        match result {
            Ok(x) => Ok(x),
            Err(_y) => Err(String::from("error loading blended variables"))
        }
    }

}
