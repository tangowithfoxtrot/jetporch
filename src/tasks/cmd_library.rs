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

// this is here to prevent typos in module code between Query & Modify 
// match legs. 

use crate::inventory::hosts::HostOSType;
use crate::tasks::FileAttributesInput;
use crate::tasks::files::Recurse;

// **IMPORTANT**
//
// all commands are responsible for screening their inputs within this file
// it is **NOT** permissible to leave this up to the caller. Err on the side
// of over-filtering!
//
// most filtering should occur in the module() evaluate code by choosing
// the right template functions.
//
// any argument that allows spaces (such as paths) should be the *last*
// command in any command sequence.

pub fn screen_path(path: &str) -> Result<String,String> {
    // NOTE: this only checks paths used in commands
    let path2 = path.trim().to_string();
    let path3 = screen_general_input_strict(&path2)?;
    Ok(path3.to_string())
}

// this filtering is applied to all shell arguments in the command library below (if not, it's an error)
// but is automatically also applied to all template calls not marked _unsafe in the evaluate() stages
// of modules. We run everything twice to prevent module coding errors.

pub fn screen_general_input_strict(input: &str) -> Result<String,String> {
    let input2 = input.trim();
    let bad = vec![ ";", "{", "}", "(", ")", "<", ">", "&", "*", "|", "=", "?", "[", "]", "$", "%", "`"];

    for invalid in bad.iter() {
        if input2.contains(invalid) {
            return Err(format!("illegal characters found: {} ('{}')", input2, invalid));
        }
    }
    Ok(input2.to_string())
}

// a slightly lighter version of checking, that allows = signs and such
// this is applied across all commands executed by the system, not just per-parameter checks
// unless run_unsafe is used internally. It is assumed that all inputs going into this command
// (parameters) are already sufficiently screened for things that can break shell commands and arguments
// are already quoted.

pub fn screen_general_input_loose(input: &str) -> Result<String,String> {
    let input2 = input.trim();
    let bad = [";", "<", ">", "&", "*", "?", "{", "}", "[", "]", "$", "`"];
    for invalid in bad.iter() {
        if input2.contains(invalid) {
            return Err(format!("illegal characters detected: {} ('{}')", input2, invalid));
        }
    }
    Ok(input2.to_string())
}

// require that octal inputs be ... octal

pub fn screen_mode(mode: &str) -> Result<String,String> {
    if FileAttributesInput::is_octal_string(mode) {
        Ok(mode.to_owned())
    } else {
        Err(format!("not an octal string: {}", mode))
    }
}

pub fn get_mode_command(os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    match os_type {
        HostOSType::Linux => Ok(format!("stat --format '%a' '{}'", path)),
        HostOSType::MacOS => Ok(format!("stat -f '%A' '{}'", path)),
    }
}

pub fn get_sha512_command(os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    match os_type {
        HostOSType::Linux => Ok(format!("sha512sum '{}'", path)),
        HostOSType::MacOS => Ok(format!("shasum -b -a 512 '{}'", path)),
    }
}

pub fn get_ownership_command(_os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    Ok(format!("ls -ld '{}'", path))
}

pub fn get_is_directory_command(_os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    Ok(format!("ls -ld '{}'", path))
}

pub fn get_touch_command(_os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    Ok(format!("touch '{}'", path))
}

pub fn get_create_directory_command(_os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    Ok(format!("mkdir -p '{}'", path))
}

pub fn get_delete_file_command(_os_type: HostOSType, untrusted_path: &str) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    Ok(format!("rm -f '{}'", path))
}

pub fn get_delete_directory_command(_os_type: HostOSType, untrusted_path: &str, recurse: Recurse) -> Result<String,String>  {
    let path = screen_path(untrusted_path)?;
    match recurse {
        Recurse::No  => { Ok(format!("rmdir '{}'", path))},
        Recurse::Yes => { Ok(format!("rm -rf '{}'", path))}
    }
}

pub fn set_owner_command(_os_type: HostOSType, untrusted_path: &str, untrusted_owner: &str, recurse: Recurse) -> Result<String,String> {
    let path = screen_path(untrusted_path)?;
    let owner = screen_general_input_strict(untrusted_owner)?;
    match recurse {
        Recurse::No   => { Ok(format!("chown '{}' '{}'", owner, path))},
        Recurse::Yes  => { Ok(format!("chown -R '{}' '{}'", owner, path))}
    }
}

pub fn set_group_command(_os_type: HostOSType, untrusted_path: &str, untrusted_group: &str, recurse: Recurse) -> Result<String,String> {
    let path = screen_path(untrusted_path)?;
    let group = screen_general_input_strict(untrusted_group)?;
    match recurse {
        Recurse::No   => { Ok(format!("chgrp '{}' '{}'", group, path))},
        Recurse::Yes  => { Ok(format!("chgrp -R '{}' '{}'", group, path))}
    }
}

pub fn set_mode_command(_os_type: HostOSType, untrusted_path: &str, untrusted_mode: &str, recurse: Recurse) -> Result<String,String> {
    // mode generally does not have to be screened but someone could call a command directly without going through FileAttributes
    // so let's be thorough.
    let path = screen_path(untrusted_path)?;
    let mode = screen_mode(untrusted_mode)?;
    match recurse {
        Recurse::No  => { Ok(format!("chmod '{}' '{}'", mode, path))},
        Recurse::Yes => { Ok(format!("chmod -R '{}' '{}'", mode, path))}
    }
}

pub fn get_arch_command(os_type: HostOSType) -> Result<String, String> {
    #[allow(clippy::match_single_binding)] // TODO: what was the intention of passing in os_type?
    match os_type {
        _ => { Ok(String::from("uname -m")) },
    }
}




