use std::str::FromStr;

#[derive(Debug)]
pub enum ChatCommands {
    Message { sender: String, message: String },
    UserConnected { name: String },
    UserDisconnected { name: String },
    UserRenamed { oldname: String, newname: String },
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

                "c" | "connect" => Ok(Self::UserConnected {
                    name: b.to_string(),
                }),
                "d" | "disconnect" => Ok(Self::UserDisconnected {
                    name: b.to_string(),
                }),

                "r" | "rename" => {
                    let (a, b) = b.split_once(' ').ok_or(())?;

                    Ok(Self::UserRenamed {
                        oldname: a.to_string(),
                        newname: b.to_string(),
                    })
                }

                _ => Err(()),
            },

            None => Err(()),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum ConnectState {
    Loading,
    Connected,
    Disconnect,
    Failed,
}

#[derive(Debug)]
pub enum ClientCommands {
    ChatCommand(ChatCommands),
    ConnectState(ConnectState),
}
