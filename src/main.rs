use hypr_some_events::{Config, Event};
use hyprland::data::Workspace;
use hyprland::event_listener::EventListenerMutable as EventListener;
use hyprland::prelude::*;
use serde_json;
use std::error::Error;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::{env, process};

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

fn subscribe_to_workspace() -> hyprland::Result<()> {
    // Display one time and retrieve active ws id
    let first_result = display_workspaces_maybe(&None);

    // Keep id from last active ws in sync
    let last_active = Arc::new(Mutex::new(Some(first_result)));
    let last_active_a = last_active.clone();
    let last_active_b = last_active.clone();
    let last_active_c = last_active.clone();
    let last_active_d = last_active.clone();
    let last_active_e = last_active.clone();

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

    // and execute the function
    // here we are using the blocking variant
    // but there is a async version too
    event_listener.start_listener()
}

fn subscribe_to_submap() -> hyprland::Result<()> {

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

fn run(config: Config) -> Result<(), Box<dyn Error>> {
    match config.event {
        Event::Workspace => subscribe_to_workspace()?,
        Event::Submap => subscribe_to_submap()?,
        Event::Invalid => eprintln!("Invalid argument")
    };

    Ok(())
}

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {err}");
        process::exit(1);
    });

    // run application
    if let Err(e) = run(config) {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
