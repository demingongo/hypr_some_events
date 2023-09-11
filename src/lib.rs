pub enum Event {
    Workspace,
    Submap,
    Invalid
}

pub struct Config {
    pub event: Event
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        // unnecessary first arg
        args.next();

        let mut extracted_args: Vec<String> = vec![];

        for arg in args {
            if arg.starts_with("--") {
            } else {
                // it's an argument
                extracted_args.push(arg);
            }
        }

        let mut extracted_args_iter = extracted_args.into_iter();

        let event = match extracted_args_iter.next() {
            Some(v) => {
                if v == "workspace" {
                    Event::Workspace
                } else if v == "submap" {
                    Event::Submap
                } else {
                    Event::Invalid
                }
            },
            None => Event::Workspace,
        };

        Ok(Config {
            event
        })
    }
}
