use chrono::offset;
use nom::{
    bytes::complete::{tag, take_till1, take_while1},
    character::{complete::space0, is_newline},
    combinator::opt,
    sequence::terminated,
    IResult,
};
use sha1_smol::Sha1;
use std::fmt;
use std::str::from_utf8;

use super::{generic_nom_failure, AsBytes};
use crate::error as err;

fn parse_seperator_line(input: &[u8]) -> IResult<&[u8], &[u8]> {
    let (input, _) = space0(input)?;
    let (input, nl) = take_while1(is_newline)(input)?;
    Ok((input, nl))
}

pub fn create_dummy_user() -> User {
    let local = offset::Local::now();
    let local_tz = local.offset().to_string().replace(":", "");
    let local_ts = local.timestamp().to_string();
    User {
        name: "foo_name".to_string(),
        email: "<foo@email.com>".to_string(),
        timestamp: format!("{} {}", local_ts, local_tz),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub name: String,
    pub email: String,
    pub timestamp: String,
}

impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{} {} {}", self.name, self.email, self.timestamp)
    }
}

fn take_till_sep_convert_val_to_string(
    separator: &'static str,
) -> impl Fn(&[u8]) -> IResult<&[u8], String> {
    move |input| {
        let sep_char = match separator {
            " " => b' ',
            "\n" => b'\n',
            _ => {
                println!("this func only supports space and newline separators");
                return Err(generic_nom_failure(input));
            }
        };
        let (input, target) = terminated(take_till1(|c| c == sep_char), tag(separator))(input)?;
        let target = match from_utf8(target) {
            Ok(t) => t.trim(),
            _ => return Err(generic_nom_failure(input)),
        };
        Ok((input, target.to_owned()))
    }
}

fn parse_user_bytes(input: &[u8]) -> IResult<&[u8], User> {
    let (input, name) = take_till_sep_convert_val_to_string(" ")(input)?;
    let (input, email) = take_till_sep_convert_val_to_string(" ")(input)?;
    let (input, timestamp) = take_till_sep_convert_val_to_string("\n")(input)?;
    Ok((
        input,
        User {
            name,
            email,
            timestamp,
        },
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Commit {
    pub tree: String,
    pub parent: Option<String>,
    pub author: User,
    pub committer: User,
    pub msg: String,
    pub sha: String,
}

impl Commit {
    /// this function is needed when creating a new Commit object
    /// vs. reading an existing one from the object store. In the
    /// case of reading an existing object the sha is already known
    pub fn calc_and_update_sha(&mut self) -> Commit {
        let mut hasher = Sha1::new();
        hasher.update(&self.as_bytes());
        let sha = hasher.digest().to_string();
        self.sha = sha;
        self.to_owned()
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if let Some(p) = &self.parent {
            write!(
                f,
                "tree {}\nparent {}\nauthor {}committer {}\n{}",
                self.tree,
                p,
                format!("{}", self.author),
                format!("{}", self.committer),
                self.msg
            )
        } else {
            write!(
                f,
                "tree {}\nauthor {}committer {}\n{}",
                self.tree,
                format!("{}", self.author),
                format!("{}", self.committer),
                self.msg
            )
        }
    }
}

impl AsBytes for Commit {
    fn as_bytes(&self) -> Vec<u8> {
        let mut commit_body = format!("{}", self).as_bytes().to_vec();
        let mut output_bytes: Vec<u8> = [
            b"commit".to_vec(),
            [b' '].to_vec(),
            commit_body.len().to_string().as_bytes().to_vec(),
            [b'\x00'].to_vec(),
        ]
        .into_iter()
        .flatten()
        .collect();

        output_bytes.append(&mut commit_body);
        output_bytes
    }
}

/// Takes a key as a str and returns a nom compatible fn. The returned fn will
/// consume the key and then capture the following value converting it to a
/// String stripping any surrounding whitespace or newlines
/// e.g. fn("tree") called with "tree sha123\n" returns ([], "sha123".to_string())
fn parse_kv_pair_v_to_string(key: &'static str) -> impl Fn(&[u8]) -> IResult<&[u8], String> {
    move |input| {
        let (input, _) = tag(key)(input)?;
        let (input, val) = terminated(take_till1(is_newline), tag("\n"))(input)?;
        let val = match from_utf8(val) {
            Ok(v) => v.trim(),
            _ => return Err(generic_nom_failure(input)),
        };
        Ok((input, val.to_owned()))
    }
}

pub fn parse_commit(input: &[u8], sha: &str) -> Result<Commit, err::Error> {
    let (input, tree) = parse_kv_pair_v_to_string("tree")(input)?;
    let (input, parent) = opt(parse_kv_pair_v_to_string("parent"))(input)?;
    let (input, _author_tag) = tag("author ")(input)?;
    let (input, author) = parse_user_bytes(input)?;
    let (input, _committer_tag) = tag("committer ")(input)?;
    let (input, committer) = parse_user_bytes(input)?;
    let (input, _) = parse_seperator_line(input)?;
    let msg = from_utf8(input)?;

    Ok(Commit {
        tree,
        parent,
        author,
        committer,
        msg: msg.to_owned(),
        sha: sha.to_owned(),
    })
}

#[cfg(test)]
mod commit_tests {
    use super::*;
    use chrono::{Local, TimeZone};

    // commit parsing test covered in object/mod.rs tests

    #[test]
    fn can_parse_user() {
        let user_bytes = [
            90, 97, 107, 45, 75, 101, 110, 116, 32, 60, 122, 97, 107, 46, 107, 101, 110, 116, 64,
            103, 109, 97, 105, 108, 46, 99, 111, 109, 62, 32, 49, 54, 55, 52, 57, 51, 57, 56, 57,
            55, 32, 45, 48, 55, 48, 48, 10,
        ];
        let local = Local
            .datetime_from_str("2023-01-28T14:04:57", "%Y-%m-%dT%H:%M:%S")
            .unwrap();
        let local_tz = local.offset().to_string().replace(":", "");
        let local_ts = local.timestamp().to_string();

        let expected_user = User {
            name: "Zak-Kent".to_string(),
            email: "<zak.kent@gmail.com>".to_string(),
            timestamp: format!("{} {}", local_ts, local_tz),
        };

        let (_, user) = parse_user_bytes(&user_bytes).unwrap();
        assert_eq!(expected_user, user);

        // checking round trip of bytes
        assert_eq!(user_bytes, format!("{}", user).as_bytes());
    }
}
