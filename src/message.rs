use std::iter::Iterator;
use std::string::ToString;
use std::str::FromStr;

/// Messages to pass to and from client handlers
#[derive(Debug)]
#[derive(Clone)]
pub struct IrcMessage {
    pub prefix: Option<String>,
    pub command: String,
    pub params: Vec<String>,
    pub trail: Option<String>,
}

impl IrcMessage {
    pub fn new<T: Into<String>>(
        prefix: Option<T>,
        command: T,
        params: Vec<T>,
        trail: Option<T>,
    ) -> IrcMessage {
        let prefix = match prefix {
            Some(prefix) => Some(prefix.into()),
            None => None,
        };
        let command = command.into();
        let params = params.into_iter().map(|param| { param.into() }).collect();
        let trail = match trail {
            Some(trail) => Some(trail.into()),
            None => None,
        };

        IrcMessage {
            prefix: prefix,
            command: command,
            params: params,
            trail: trail,
        }
    }
}

impl ToString for IrcMessage {
    fn to_string(&self) -> String {
        let mut s = "".to_string();
        if let Some(ref prefix) = self.prefix {
            s = s + ":" + prefix + " ";
        }
        s = s + &self.command;
        for param in self.params.iter() {
            s = s + " " + param;
        }
        if let Some(ref trail) = self.trail {
            s = s + " :" + trail;
        }
        s
    }
}

impl FromStr for IrcMessage {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let words: Vec<&str> = s.trim_right_matches("\r").split(" ").collect();
        let mut i = 0;

        let prefix = if words[i].slice_chars(0, 1) == ":" {
            i += 1;
            Some(words[i - 1].slice_chars(1, words[i - 1].len()).to_string())
        } else {
            None
        };

        let command = words[i].to_string();
        i += 1;

        let mut params = vec![];
        let mut trail = None;

        while i < words.len() {
            if words[i].slice_chars(0, 1) == ":" {
                let mut trail_parts = words[i].slice_chars(1, words[i].len()).to_string();
                for trail_part in &words[(i + 1)..] {
                    trail_parts.push(' ');
                    trail_parts = trail_parts + trail_part;
                }
                trail = Some(trail_parts);
                break;
            } else {
                params.push(words[i].to_string());
                i += 1;
            }
        }

        Ok(IrcMessage {
            prefix: prefix,
            command: command,
            params: params,
            trail: trail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::IrcMessage;

    #[test]
    fn new_works() {
        let msg = IrcMessage::new(Some("mythmon"), "PRIVMSG", vec!["#osu-lug"], Some("It works!"));

        assert_eq!(msg.prefix, Some("mythmon".to_string()));
        assert_eq!(msg.command, "PRIVMSG".to_string());
        assert_eq!(msg.params, vec!["#osu-lug".to_string()]);
        assert_eq!(msg.trail, Some("It works!".to_string()));
    }

    #[test]
    fn to_string_full() {
        let msg = IrcMessage::new(Some("mythmon"), "PRIVMSG", vec!["#osu-lug"], Some("It works!"));
        let s = msg.to_string();
        assert_eq!(s, ":mythmon PRIVMSG #osu-lug :It works!");
    }

    #[test]
    fn to_string_no_prefix() {
        let msg = IrcMessage::new(None, "PRIVMSG", vec!["mythmon"], Some("yt?"));
        let s = msg.to_string();
        assert_eq!(s, "PRIVMSG mythmon :yt?");
    }

    #[test]
    fn to_string_no_trail_no_prefix() {
        let msg = IrcMessage::new(None, "PING", vec!["freenode"], None);
        let s = msg.to_string();
        assert_eq!(s, "PING freenode");
    }

    #[test]
    fn to_string_multiple_args() {
        let msg = IrcMessage::new(None, "USER", vec!["mythmon", "mythmon", "localhost"], Some("Unknown"));
        let s = msg.to_string();
        assert_eq!(s, "USER mythmon mythmon localhost :Unknown");
    }

    #[test]
    fn parse_full() {
        let msg: IrcMessage = ":mythmon PRIVMSG #osu-lug :It works!".parse().unwrap();

        assert_eq!(msg.prefix, Some("mythmon".to_string()));
        assert_eq!(msg.command, "PRIVMSG".to_string());
        assert_eq!(msg.params, vec!["#osu-lug".to_string()]);
        assert_eq!(msg.trail, Some("It works!".to_string()));
    }

    #[test]
    fn parse_no_prefix() {
        let msg: IrcMessage = "PRIVMSG mythmon :It works!".parse().unwrap();

        assert_eq!(msg.prefix, None);
        assert_eq!(msg.command, "PRIVMSG".to_string());
        assert_eq!(msg.params, vec!["mythmon".to_string()]);
        assert_eq!(msg.trail, Some("It works!".to_string()));
    }

    #[test]
    fn parse_no_trail_no_prefix() {
        let msg: IrcMessage = "PING freenode".parse().unwrap();

        assert_eq!(msg.prefix, None);
        assert_eq!(msg.command, "PING".to_string());
        assert_eq!(msg.params, vec!["freenode".to_string()]);
        assert_eq!(msg.trail, None);
    }

    #[test]
    fn parse_multiple_args() {
        let msg: IrcMessage = "USER mythmon mythmon localhost :Unknown".parse().unwrap();

        assert_eq!(msg.prefix, None);
        assert_eq!(msg.command, "USER".to_string());
        assert_eq!(msg.params, vec!["mythmon".to_string(), "mythmon".to_string(), "localhost".to_string()]);
        assert_eq!(msg.trail, Some("Unknown".to_string()));
    }

    #[test]
    fn parse_numeric_command() {
        let msg: IrcMessage = "001 :Welcome".parse().unwrap();

        assert_eq!(msg.prefix, None);
        assert_eq!(msg.command, "001".to_string());
        assert_eq!(msg.params.len(), 0);
        assert_eq!(msg.trail, Some("Welcome".to_string()));
    }

    #[test]
    fn round_tripping() {
        let messages = vec![
            ":sendak.freenode.net NOTICE * :*** Looking up your hostname...",
            "NICK mythmon",
            ":sendak.freenode.net 433 * mythmon :Nickname is already in use.",
            "NICK notmythmon",
            "USER notmythmon notmythmon freenode :Unknown",
            ":sendak.freenode.net 001 notmythmon :Welcome to the freenode Internet Relay Chat Network notmythmon",
            ":sendak.freenode.net 005 notmythmon EXTBAN=$,ajrxz WHOX CLIENTVER=3.0 SAFELIST ELIST=CTU :are supported by this server",
            ":sendak.freenode.net 251 notmythmon :There are 164 users and 85708 invisible on 27 servers",
            ":sendak.freenode.net 252 notmythmon 22 :IRC Operators online",
            ":sendak.freenode.net 253 notmythmon 11 :unknown connection(s)",
            ":sendak.freenode.net 254 notmythmon 58958 :channels formed",
            ":sendak.freenode.net 255 notmythmon :I have 5770 clients and 1 servers",
            ":sendak.freenode.net 265 notmythmon 5770 8825 :Current local users 5770, max 8825",
            ":sendak.freenode.net 266 notmythmon 85872 99340 :Current global users 85872, max 99340",
            ":sendak.freenode.net 375 notmythmon :- sendak.freenode.net Message of the Day -",
            ":sendak.freenode.net 372 notmythmon :- Welcome to sendak.freenode.net in Vilnius, Lithuania, EU.",
            ":sendak.freenode.net 376 notmythmon :End of /MOTD command.",
            ":notmythmon MODE notmythmon :+i",
            "JOIN #osu-lug",
            ":notmythmon!~notmythmo@2602:47:20f4:5100:7e7a:91ff:fe86:952a JOIN #osu-lug",
            ":sendak.freenode.net 332 notmythmon #osu-lug :Oregon State University Linux Users Group :: 6pm Tuesdays KEC1007 :: http://lug.oregonstate.edu :: #osu-lug-admin for bureaucracy :: Freenode policies apply -- http://freenode.net/policy.shtml :: Officer Nominations http://bit.ly/1IvEJov :: CHOVDA - Youngs @ Noon",
            ":sendak.freenode.net 333 notmythmon #osu-lug pop 1433522980",
            ":sendak.freenode.net 353 notmythmon = #osu-lug :notmythmon lleu @hamper magical chetco jesusaurus teiresias +chekkaa zerocool Sir-Batman akeym psandin Ramereth darkengine Ac-town cash-override terrellt runawayfive Jennjitzu thai pono wenzel vanrysss leel8on spectralsun Alan_S_ pwnguin nibalizer currymi zubriske recyclops borcean pruittt Mathuin bryon jme robatron lrr patcht frostsnow Wahrheit jnoah Rhodops bkero lamereb bramwelt vidkjerd_ merdler vidkjerd spaceships mburns edunham tschuy +pop",
            ":sendak.freenode.net 353 notmythmon = #osu-lug :pingveno chriconn_ [VartanK1 chance Orcbane +radens shieldal chapmant3 +deanj blkperl mythmon vandykel murrown cmartin0 shawl farryr1 +lucyw hoangt relud Jeff_S armoredPotato chetco__ Odysimus ekem MaraJade tolvstaa leian2 scooley_ irdan scooley demophoon marineam localhost00 smcgregor voidpris1",
            ":sendak.freenode.net 366 notmythmon #osu-lug :End of /NAMES list.",
            "PING freenode",
            ":sendak.freenode.net PONG sendak.freenode.net :freenode",
            ":mythmon!~mythmon@osuosl/staff/Mythmon PRIVMSG notmythmon :test",
            "PRIVMSG mythmon :yes?",
        ];

        for msg in messages.into_iter() {
            assert_eq!(msg.to_string(), msg.parse::<IrcMessage>().unwrap().to_string());
        }
    }
}
