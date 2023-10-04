use hyprland::data::Workspace;
use hyprland::event_listener::EventListenerMutable as EventListener;
use hyprland::prelude::*;
use serde_json;
use std::error::Error;
use std::process::Command;
use std::sync::{Arc, Mutex};

const EWW_CMD: &str = "eww";

pub enum Event {
    Workspace,
    ActiveWorkspace,
    Submap,
    Invalid
}

pub struct Config {
    pub event: Event,
    pub ewwvar: String
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        // unnecessary first arg
        args.next();

        let mut extracted_args: Vec<String> = vec![];
        let mut options: Vec<String> = vec![];

        for arg in args {
            if arg.starts_with("--") {
                options.push(arg);
            } else {
                // it's an argument
                extracted_args.push(arg);
            }
        }

        let mut extracted_args_iter = extracted_args.into_iter();
        let options_iter = options.into_iter();
        
        let event = match extracted_args_iter.next() {
            Some(v) => {
                if v == "workspace" || v == "workspaces" {
                    Event::Workspace
                } else if v == "active-workspace" {
                    Event::ActiveWorkspace
                } else if v == "submap" {
                    Event::Submap
                } else {
                    Event::Invalid
                }
            },
            None => Event::Workspace,
        };

        let mut ewwvar = String::new();

        for arg in options_iter {
            if arg.starts_with("--eww=") {
                ewwvar = String::from(&arg[6..]);
            }
        }

        Ok(Config {
            event,
            ewwvar
        })
    }
}

fn exec_hyprctl_command(hyprctl_command: &str) -> serde_json::Value {
    let output = Command::new("hyprctl")
        .arg("-j")
        .arg(hyprctl_command)
        .output()
        .unwrap();

    let output_string = String::from_utf8(output.stdout).unwrap();

    serde_json::from_str(output_string.as_str()).unwrap()
}

fn get_workspaces_list(active_id: i64) -> serde_json::Value {
    // get list of workspaces
    let mut object = exec_hyprctl_command("workspaces");
    let workspaces = object.as_array_mut();

    if let Some(workspaces_array) = workspaces {

        // sort workspaces
        workspaces_array.sort_by(|a, b| {
            let a_id = a.get("id").unwrap().as_i64().unwrap();
            let b_id = b.get("id").unwrap().as_i64().unwrap();
            a_id.partial_cmp(&b_id).unwrap()
        });

        // loop through list
        for work in workspaces_array {
            // get workspace
            let work_json = work.as_object_mut();
            if let Some(ev) = work_json {
                // get workspace id
                let value_option = ev.get_mut("id");
                if let Some(value) = value_option {
                    if let serde_json::Value::Number(num) = value {
                        if let Some(num_value) = num.as_i64() {
                            // add property "num" with same value from property "id"
                            ev.insert(
                                String::from("num"),
                                serde_json::to_value(num_value).unwrap(),
                            );

                            // add property "focused": true if "id" equals the function's argument
                            let mut focused = false;
                            if num_value == active_id {
                                focused = true;
                            }
                            ev.insert(String::from("focused"), serde_json::Value::Bool(focused));
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("error - workspace list is not a JSON array!");
    }

    object
}

fn get_number(option_value: Option<&mut serde_json::Value>) -> Option<i64> {
    if let Some(value) = option_value {
        if let serde_json::Value::Number(num) = value {
            if let Some(num_value) = num.as_i64() {
                return Some(num_value)
            }
        }
    }
    None
}

fn assign_map<'a>(target: &'a mut serde_json::Map<String, serde_json::Value>, source: &serde_json::Map<String, serde_json::Value>) -> &'a mut serde_json::Map<String, serde_json::Value>{
    for (k, v) in source {
        target.insert(k.to_string(), v.clone());
    }
    target
}

fn get_persistent_workspaces_list(persistent_workspaces: Vec<serde_json::Value>, active_id: i64) -> serde_json::Value {
    // get list of workspaces
    let mut object = exec_hyprctl_command("workspaces");
    let workspaces = object.as_array_mut();

    let mut result: Vec<serde_json::Value> = vec![];

    if let Some(workspaces_array) = workspaces {

        // sort workspaces
        workspaces_array.sort_by(|a, b| {
            let a_id = a.get("id").unwrap().as_i64().unwrap();
            let b_id = b.get("id").unwrap().as_i64().unwrap();
            a_id.partial_cmp(&b_id).unwrap()
        });

        // iteration/loop through persistent list
        for mut persistent_work in persistent_workspaces {
            let persistent_work_json = persistent_work.as_object_mut();
            if let Some(pwork_map) = persistent_work_json {
                let pid_option = get_number(pwork_map.get_mut("id"));
                let mut map_to_insert = pwork_map;
                let mut is_active = false;
                if let Some(pid) = pid_option {
                    
                    // fresh borrow "workspaces_array" to use inside of iteration loop
                    for work in &mut *workspaces_array {
                        // get workspace
                        let work_json = work.as_object_mut();
                        if let Some(work_map) = work_json {
                            // get workspace id
                            let id_option = get_number(work_map.get_mut("id"));
                            if let Some(id) = id_option {
                                if pid == id {
                                    // clone, mix and break
                                    map_to_insert = assign_map(work_map, map_to_insert);
                                    is_active = true;
                                    break;
                                }
                            }
                        }
                    }

                    map_to_insert.insert(
                        String::from("num"),
                        serde_json::to_value(pid).unwrap(),
                    );

                    map_to_insert.insert(String::from("active"), serde_json::Value::Bool(is_active));

                    // add property "focused": true if "id" equals the function's argument
                    let mut focused = false;
                    if pid == active_id {
                        focused = true;
                    }
                    map_to_insert.insert(String::from("focused"), serde_json::Value::Bool(focused));

                    result.push(serde_json::to_value(map_to_insert).unwrap());
                }
            }

            //result.push(persistent_work);
        }
    } else {
        eprintln!("error - workspace list is not a JSON array!");
        for persistent_work in persistent_workspaces {
            result.push(persistent_work);
        }
    }

    serde_json::from_value(serde_json::Value::Array(result)).unwrap()
}

fn get_active_workspace_id() -> Option<i32> {
    let result = Workspace::get_active();
    match result {
        Ok(work) => Some(work.id),
        err => {
            eprintln!("error get_active_workspace_id: {err:?}");
            None
        }
    }
}

/// Executes "eww get <ewwvar>" and expects the output to be 
/// a JSON array of objects { "num", "name" }
fn get_ewwvar_workspaces(ewwvar: &String) -> Option<Vec<serde_json::Value>> {
    let mut binding = std::process::Command::new(EWW_CMD);
    let result = binding
        .arg("get")
        .arg(ewwvar)
        .output();

    match result {
        Ok(output) => {
            let output_string = String::from_utf8(output.stdout).unwrap();
            match serde_json::from_str(output_string.as_str()) {
                Ok(output_value) => {
                    if let serde_json::Value::Array(output_vec) = output_value {
                        return Some(output_vec)
                    } else {
                        return None
                    }
                },
                Err(e) => {
                    eprintln!("Could not parse value of eww var {:?}: {:?}", ewwvar, e);
                    None
                }
            }
        },
        Err(e) => {
            eprintln!("Could not execute command: eww get {:?}: {:?}", ewwvar, e);
            None
        }
    }
}

/// Displays workspaces as JSON if new (focused) workspaces.
fn display_persistent_workspaces_maybe(previous_active_ws_id: &Option<i32>, persistent_workspaces: Vec<serde_json::Value>) -> i32 {
    let work_id = get_active_workspace_id();

    let default_ws_id = i64::from(0);

    match work_id {
        Some(id) => {
            if let Some(arg_id) = previous_active_ws_id {
                if *arg_id != id {
                    let result = get_persistent_workspaces_list(persistent_workspaces, i64::from(id)).to_string();
                    if !result.is_empty() {
                        println!("{}", result);
                        //println!("Display now!");
                    }
                } else {
                    eprintln!("Still same workspace");
                }
            } else {
                let result = get_persistent_workspaces_list(persistent_workspaces, i64::from(id)).to_string();
                if !result.is_empty() {
                    println!("{}", result);
                    //println!("Display now!");
                }
            }
            id
        }
        None => {
            eprintln!("error - could not get active workspace");
            let result = get_workspaces_list(default_ws_id).to_string();
            if !result.is_empty() {
                println!("{}", result);
                //println!("Display now!");
            }
            i32::from(0)
        }
    }
}

/// Displays workspaces as JSON if new (focused) workspaces.
fn display_workspaces_maybe(previous_active_ws_id: &Option<i32>) -> i32 {
    let work_id = get_active_workspace_id();

    let default_ws_id = i64::from(0);

    match work_id {
        Some(id) => {
            if let Some(arg_id) = previous_active_ws_id {
                if *arg_id != id {
                    let result = get_workspaces_list(i64::from(id)).to_string();
                    if !result.is_empty() {
                        println!("{}", result);
                        //println!("Display now!");
                    }
                } else {
                    eprintln!("Still same workspace");
                }
            } else {
                let result = get_workspaces_list(i64::from(id)).to_string();
                if !result.is_empty() {
                    println!("{}", result);
                    //println!("Display now!");
                }
            }
            id
        }
        None => {
            eprintln!("error - could not get active workspace");
            let result = get_workspaces_list(default_ws_id).to_string();
            if !result.is_empty() {
                println!("{}", result);
                //println!("Display now!");
            }
            i32::from(0)
        }
    }
}

/// Displays active workspace as JSON if new (focused) workspace.
fn display_active_workspace_maybe(previous_active_ws_id: &Option<i32>) -> i32 {
    let result = Workspace::get_active();
    let workspace = match result {
        Ok(work) => Some(work),
        err => {
            eprintln!("error display_active_workspace_maybe: {err:?}");
            None
        }
    };

    if let Some(work) = workspace {
        if let Some(arg_id) = previous_active_ws_id {
            if *arg_id != work.id {
                println!("{}", work.id);
            } else {
                eprintln!("Still same workspace");
            }
        } else {
            println!("{}", work.id);
        }
        return work.id
    } else {
        println!("{}", 0);
        return i32::from(0)
    }
}

pub fn subscribe_to_workspace_eww(ewwvar: String) -> hyprland::Result<()> {

    // Expect a Vec of serde_json::Value (objects)
    let ewwvar_value = Arc::new(Mutex::new(get_ewwvar_workspaces(&ewwvar)));
    let ewwvar_value_a = ewwvar_value.clone();
    let ewwvar_value_b = ewwvar_value.clone();
    let ewwvar_value_c = ewwvar_value.clone();
    let ewwvar_value_d = ewwvar_value.clone();
    let ewwvar_value_e = ewwvar_value.clone();
    let ewwvar_value_f = ewwvar_value.clone();

    // Display one time and retrieve active ws id
    let first_result = match &ewwvar_value.lock().unwrap().as_deref() {
        Some(v) => display_persistent_workspaces_maybe(&None, v.to_vec()),
        None => display_workspaces_maybe(&None)
    };

    // Keep id from last active ws in sync
    let last_active = Arc::new(Mutex::new(Some(first_result)));
    let last_active_a = last_active.clone();
    let last_active_b = last_active.clone();
    let last_active_c = last_active.clone();
    let last_active_d = last_active.clone();
    let last_active_e = last_active.clone();
    let last_active_f = last_active.clone();

    // Create a event listener
    let mut event_listener = EventListener::new();

    // Shows when active window changes
    event_listener.add_active_window_change_handler(move |_, _| {
        let result = match &ewwvar_value_a.lock().unwrap().as_deref() {
            Some(v) => display_persistent_workspaces_maybe(&last_active_a.lock().unwrap(), v.to_vec()),
            None => display_workspaces_maybe(&last_active_a.lock().unwrap())
        };
        last_active_a.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_change_handler(move |_, _| {
        let result = match &ewwvar_value_b.lock().unwrap().as_deref() {
            Some(v) => display_persistent_workspaces_maybe(&None, v.to_vec()),
            None => display_workspaces_maybe(&None)
        };
        last_active_b.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_added_handler(move |_, _| {
        let result = match &ewwvar_value_c.lock().unwrap().as_deref() {
            Some(v) => display_persistent_workspaces_maybe(&None, v.to_vec()),
            None => display_workspaces_maybe(&None)
        };
        last_active_c.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_moved_handler(move |_, _| {
        let result = match &ewwvar_value_d.lock().unwrap().as_deref() {
            Some(v) => display_persistent_workspaces_maybe(&None, v.to_vec()),
            None => display_workspaces_maybe(&None)
        };
        last_active_d.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_destroy_handler(move |_, _| {
        let result = match &ewwvar_value_e.lock().unwrap().as_deref() {
            Some(v) => display_persistent_workspaces_maybe(&None, v.to_vec()),
            None => display_workspaces_maybe(&None)
        };
        last_active_e.lock().unwrap().replace(result);
    });

    // monitor change
    event_listener.add_active_monitor_change_handler(move |_, _| {
        let result = match &ewwvar_value_f.lock().unwrap().as_deref() {
            Some(v) => display_persistent_workspaces_maybe(&last_active_f.lock().unwrap(), v.to_vec()),
            None => display_workspaces_maybe(&None)
        };
        last_active_f.lock().unwrap().replace(result);
    });

    // and execute the function
    // here we are using the blocking variant
    // but there is a async version too
    event_listener.start_listener()
}

pub fn subscribe_to_workspace() -> hyprland::Result<()> {
    // Display one time and retrieve active ws id
    let first_result = display_workspaces_maybe(&None);

    // Keep id from last active ws in sync
    let last_active = Arc::new(Mutex::new(Some(first_result)));
    let last_active_a = last_active.clone();
    let last_active_b = last_active.clone();
    let last_active_c = last_active.clone();
    let last_active_d = last_active.clone();
    let last_active_e = last_active.clone();
    let last_active_f = last_active.clone();

    // Create a event listener
    let mut event_listener = EventListener::new();

    // Shows when active window changes
    event_listener.add_active_window_change_handler(move |_, _| {
        let result = display_workspaces_maybe(&last_active_a.lock().unwrap());
        last_active_a.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_change_handler(move |_, _| {
        let result = display_workspaces_maybe(&None);
        last_active_b.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_added_handler(move |_, _| {
        let result = display_workspaces_maybe(&None);
        last_active_c.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_moved_handler(move |_, _| {
        let result = display_workspaces_maybe(&None);
        last_active_d.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_destroy_handler(move |_, _| {
        let result = display_workspaces_maybe(&None);
        last_active_e.lock().unwrap().replace(result);
    });

    // monitor change
    event_listener.add_active_monitor_change_handler(move |_, _| {
        let result = display_workspaces_maybe(&last_active_f.lock().unwrap());
        last_active_f.lock().unwrap().replace(result);
    });

    // and execute the function
    // here we are using the blocking variant
    // but there is a async version too
    event_listener.start_listener()
}

pub fn subscribe_to_active_workspace() -> hyprland::Result<()> {
    // Display one time and retrieve active ws id
    let first_result = display_active_workspace_maybe(&None);

    // Keep id from last active ws in sync
    let last_active = Arc::new(Mutex::new(Some(first_result)));
    let last_active_a = last_active.clone();
    let last_active_b = last_active.clone();
    let last_active_c = last_active.clone();
    let last_active_d = last_active.clone();
    let last_active_e = last_active.clone();
    let last_active_f = last_active.clone();

    // Create a event listener
    let mut event_listener = EventListener::new();

    // Shows when active window changes
    event_listener.add_active_window_change_handler(move |_, _| {
        let result = display_active_workspace_maybe(&last_active_a.lock().unwrap());
        last_active_a.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_change_handler(move |_, _| {
        let result = display_active_workspace_maybe(&None);
        last_active_b.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_added_handler(move |_, _| {
        let result = display_active_workspace_maybe(&None);
        last_active_c.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_moved_handler(move |_, _| {
        let result = display_active_workspace_maybe(&None);
        last_active_d.lock().unwrap().replace(result);
    });

    event_listener.add_workspace_destroy_handler(move |_, _| {
        let result = display_active_workspace_maybe(&None);
        last_active_e.lock().unwrap().replace(result);
    });

    // monitor change
    event_listener.add_active_monitor_change_handler(move |_, _| {
        let result = display_active_workspace_maybe(&last_active_f.lock().unwrap());
        last_active_f.lock().unwrap().replace(result);
    });

    // and execute the function
    // here we are using the blocking variant
    // but there is a async version too
    event_listener.start_listener()
}

pub fn subscribe_to_submap() -> hyprland::Result<()> {

    // Create a event listener
    let mut event_listener = EventListener::new();

    // Shows when active window changes
    event_listener.add_sub_map_change_handler(move |value, _| {
        let mut output: serde_json::Value = serde_json::from_str("{}").unwrap();

        let mut name = String::from("default");
        if !value.is_empty() {
            name = value
        }

        let output_json = output.as_object_mut();
        if let Some(ev) = output_json {
            ev.insert(String::from("name"), serde_json::to_value(name).unwrap());
        }

        println!("{}", output.to_string())
    });

    // and execute the function
    // here we are using the blocking variant
    // but there is a async version too
    event_listener.start_listener()
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    match config.event {
        Event::Workspace => {
            if !config.ewwvar.is_empty() {
                subscribe_to_workspace_eww(config.ewwvar)?
            } else {
                subscribe_to_workspace()?
            }
        },
        Event::ActiveWorkspace => subscribe_to_active_workspace()?,
        Event::Submap => subscribe_to_submap()?,
        Event::Invalid => eprintln!("Invalid argument")
    };

    Ok(())
}