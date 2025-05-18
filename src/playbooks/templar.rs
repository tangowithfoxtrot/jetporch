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

use serde_yaml;
use once_cell::sync::Lazy;
use handlebars::{Handlebars,RenderError};

use crate::playbooks::t_helpers::register_helpers;

// templar contains low-level wrapping around handlebars.
// this is not used directly when evaluating templates and template
// expressions, for this, see handle/template.rs

static HANDLEBARS: Lazy<Handlebars> = Lazy::new(|| {
    let mut hb = Handlebars::new();
    // very important: we are not plugging variables into HTML, turn escaping off
    hb.register_escape_fn(handlebars::no_escape);
    hb.set_strict_mode(true);
    register_helpers(&mut hb);
    hb
});

// 'off' mode is used in a bit of a weird traversal/engine
// situation where we need to get access to some task parameters
// before templates are evaluated. You will notice there is no way
// to evaluate templates in unstrict mode. This is by design.

#[derive(PartialEq,Copy,Clone,Debug)]
pub enum TemplateMode {
    Strict,
    Off
}

pub struct Templar {
}

impl Templar {

    pub fn new() -> Self {
        Self {}
    }

    // evaluate a string

    pub fn render(&self, template: &str, data: serde_yaml::Mapping, template_mode: TemplateMode) -> Result<String, String> {
        let result : Result<String, RenderError> = match template_mode {
            TemplateMode::Strict => HANDLEBARS.render_template(template, &data),
            /* this is only used to get back the raw 'items' collection inside the task FSM */
            TemplateMode::Off => Ok(String::from("empty"))
        };
        match result {
            Ok(x) => {
                Ok(x)
            },
            Err(y) => {
                Err(format!("Template error: {}", y.desc))
            }
        }
    }
    
    // used for with/cond and also in the shell module

    pub fn test_condition(&self, expr: &String, data: serde_yaml::Mapping, template_mode: TemplateMode) -> Result<bool, String> {
        if template_mode == TemplateMode::Off {
            /* this is only used to get back the raw 'items' collection inside the task FSM */
            return Ok(true);
        }
        // embed the expression in an if statement as a way to evaluate it for truth
        let template = format!("{{{{#if {expr} }}}}true{{{{ else }}}}false{{{{/if}}}}");
        let result = self.render(&template, data, TemplateMode::Strict);
        match result {
            Ok(x) => { 
                if x.as_str().eq("true") {
                    Ok(true)
                } else {
                    Ok(false)
                }
            },
            Err(y) => { 
                if y.contains("Couldn't read parameter") {
                    Err(format!("failed to parse conditional: {}: one or more parameters may be undefined", expr))
                }
                else {
                    Err(format!("failed to parse conditional: {}: {}", expr, y))
                }
            }
        }
    }

}
