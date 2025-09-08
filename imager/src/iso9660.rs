
/// One of the classes of strings defined by is09660
struct Iso9660StringA(String);

/// One of the classes of strings defined by is09660
struct Iso9660StringD(String);

/// Defines what files are allowed to do on the filesystem
pub enum InterchangeLevel {
    /// Plain old dos filenames.
    /// 4GiB maximum filesize
    One,
    /// 30 characters allowed for filenames
    /// 4GiB maximum filesize
    Two,
    /// 30 characters allowed for filenames
    /// Effectively no limit on filesize
    Three,
}

impl TryFrom<&str> for Iso9660StringA {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let s = value.to_ascii_uppercase();
        for b in s.chars() {
            match b {
                'A'..='Z'
                | '0'..='9'
                | '_'
                | '!'
                | '"'
                | '&'
                | '\''
                | '('
                | ')'
                | '*'
                | '+'
                | ','
                | '-'
                | '.'
                | '/'
                | ':'
                | ';'
                | '<'
                | '='
                | '>'
                | '?' => {}
                _ => {
                    return Err(format!("Invalid character {}", b));
                }
            }
        }
        Ok(Self(s))
    }
}

impl TryFrom<&str> for Iso9660StringD {
    type Error = String;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let s = value.to_ascii_uppercase();
        for b in s.chars() {
            match b {
                'A'..='Z'
                | '0'..='9'
                | '_' => {}
                _ => {
                    return Err(format!("Invalid character {}", b));
                }
            }
        }
        Ok(Self(s))
    }
}

struct PrimaryVolume {}

impl PrimaryVolume {
    pub fn new() -> Self {
        Self {}
    }

    /// Write the volume data
    pub fn make_contents(&self) -> Vec<u8> {
        let mut contents = Vec::new();
        contents.push(1); //descriptor type
        contents.append(&mut "CD001".as_bytes().to_vec()); //standard identifier
        contents.push(1); //version
        contents.push(0); //unused
        contents.append(&mut "                                ".as_bytes().to_vec()); //system identifier

        contents
    }
}

struct SupplementaryVolume {}

struct VolumePartition {}

enum Iso9660Volume {
    Primary(PrimaryVolume),
    Supplementary(SupplementaryVolume),
    Volume(VolumePartition),
    Terminator,
}

pub struct Iso9660Image {
    sys_area: [u8; 0x8000],
}

impl Iso9660Image {
    /// construct an empty iso image
    pub fn new() -> Self {
        Self {
            sys_area: [0; 0x8000],
        }
    }

    /// Write this iso image to a file
    pub fn write_to_file(&self, s: std::path::PathBuf) -> Result<(), std::io::Error> {
        let mut f = std::fs::File::create(s)?;

        todo!()
    }
}