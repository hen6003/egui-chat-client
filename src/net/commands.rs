use std::str::FromStr;

#[derive(Debug)]
pub enum ChatCommands {
    Message { sender: String, message: String },
}

impl FromStr for ChatCommands {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s.split_once(' ') {
            Some((a, b)) => match a {
                "m" | "msg" => {
                    let (a, b) = b.split_once(' ').ok_or(())?;

                    Ok(Self::Message {
                        sender: a.to_string(),
                        message: b.to_string(),
                    })
                }

                _ => Err(()),
            },

            None => Err(()),
        }
    }
}
