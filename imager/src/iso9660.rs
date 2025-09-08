use std::io::{Read, Write};


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
                | ' '
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
                    return Err(format!("Invalid character '{}'", b));
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
                | ' '
                | '_' => {}
                _ => {
                    return Err(format!("Invalid character '{}'", b));
                }
            }
        }
        Ok(Self(s))
    }
}

struct BootVolume {
    boot_system_id: Iso9660StringA,
    boot_id: Iso9660StringA,
    custom: Vec<u8>,
}

impl TryFrom<&[u8; 2048]> for BootVolume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        let standard = str::from_utf8(&value[1..6]).map_err(|e| format!("Invalid standard in primary volume: {}", e))?.to_string();
        assert_eq!(standard, "CD001".to_string());
        assert_eq!(1, value[6]); //version
        let bsi = str::from_utf8(value[7..39].trim_ascii_end()).map_err(|e| format!("Invalid char in boot system identifier: {:?}", e))?;
        let bsi = bsi.trim_end_matches(char::from(0));
        let bi = str::from_utf8(value[39..71].trim_ascii_end()).map_err(|e| format!("Invalid char in boot identifier: {:?}", e))?;
        let bi = bi.trim_end_matches(char::from(0));
        log::info!("Boot system identifier is '{}'", bsi);
        Ok(Self {
            boot_system_id: bsi.try_into()?,
            boot_id: bi.try_into()?,
            custom: value[71..].to_vec(),
        })
    }
}

impl Iso9660VolumeTrait for BootVolume {
    fn get_contents(&self, descriptor: &mut [u8; 2048]) {
        descriptor[0] = 0;
        descriptor[2..7].copy_from_slice("CD001".as_bytes());
        descriptor[7] = 1; //version
    }
}

struct PrimaryVolume {
    /// The system identifier
    system: Iso9660StringA,
    /// Volume id
    volume_id: Iso9660StringD,
    /// Volume size in blocks
    size: u32,
    /// total number of volumes
    total_volumes: u16,
    /// number for this volume
    this_volume: u16,
}

impl TryFrom<&[u8; 2048]> for PrimaryVolume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        let standard = str::from_utf8(&value[1..6]).map_err(|e| format!("Invalid standard in primary volume: {}", e))?.to_string();
        assert_eq!(standard, "CD001".to_string());
        assert_eq!(1, value[6]); //version
        assert_eq!(0, value[7]); //unused
        let sys = str::from_utf8(value[8..40].trim_ascii_end()).map_err(|e| format!("Invalid char in system identifier: {:?}", e))?;
        let vol = str::from_utf8(value[40..72].trim_ascii_end()).map_err(|e| format!("Invalid char in volume identifier: {:?}", e))?;
        log::info!("Vol is {}", vol);
        Ok(Self {
            system: sys.try_into()?,
            volume_id: vol.try_into()?,
            size: u32::from_le_bytes(value[80..84].try_into().unwrap()),
            total_volumes: u16::from_le_bytes(value[120..122].try_into().unwrap()),
            this_volume: u16::from_le_bytes(value[124..126].try_into().unwrap()),
        })
    }
}

impl Default for PrimaryVolume {
    fn default() -> Self {
        Self {
            system: "                                ".try_into().unwrap(),
            volume_id: "DEFAULT".try_into().unwrap(),
            size: 1,
            total_volumes: 1,
            this_volume: 1,
        }
    }
}

impl Iso9660VolumeTrait for PrimaryVolume {
    fn get_contents(&self, descriptor: &mut [u8; 2048]) {
        descriptor[0] = 1;
        descriptor[2..7].copy_from_slice("CD001".as_bytes());
        descriptor[7] = 1; //version
        descriptor[8..40].copy_from_slice(format!("{:<32}", self.system.0).as_bytes());
        descriptor[40..72].copy_from_slice(format!("{:<32}", self.volume_id.0).as_bytes());
    }
}

struct SupplementaryVolume {}

impl TryFrom<&[u8; 2048]> for SupplementaryVolume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        todo!();
        Ok(Self {})
    }
}

impl Iso9660VolumeTrait for SupplementaryVolume {
    fn get_contents(&self, descriptor: &mut [u8; 2048]) {}
}

struct VolumePartition {}

impl TryFrom<&[u8; 2048]> for VolumePartition {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        todo!();
        Ok(Self {})
    }
}

impl Iso9660VolumeTrait for VolumePartition {
    fn get_contents(&self, descriptor: &mut [u8; 2048]) {}
}
struct VolumeTerminator {}

impl TryFrom<&[u8; 2048]> for VolumeTerminator {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        let standard = str::from_utf8(&value[1..6]).map_err(|e| format!("Invalid standard in primary volume: {}", e))?.to_string();
        assert_eq!(standard, "CD001".to_string());
        assert_eq!(1, value[6]); //version
        Ok(Self {})
    }
}

impl Iso9660VolumeTrait for VolumeTerminator {
    fn get_contents(&self, descriptor: &mut [u8; 2048]) {
        descriptor[0] = 255;
        descriptor[1..6].copy_from_slice("CD001".as_bytes());
        descriptor[6] = 1;
    }
}

/// The volume trait for volumes in an iso9660 image
#[enum_dispatch::enum_dispatch]
trait Iso9660VolumeTrait {
    /// Get the contents of the volume
    fn get_contents(&self, descriptor: &mut [u8; 2048]);
}

#[enum_dispatch::enum_dispatch(Iso9660VolumeTrait)]
enum Iso9660Volume {
    Boot(BootVolume),
    Primary(PrimaryVolume),
    Supplementary(SupplementaryVolume),
    Volume(VolumePartition),
    Terminator(VolumeTerminator),
}

impl TryFrom<&[u8; 2048]> for Iso9660Volume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        match value[0] {
            0 => Ok(Self::Boot(BootVolume::try_from(value)?)),
            1 => Ok(Self::Primary(PrimaryVolume::try_from(value)?)),
            2 => Ok(Self::Supplementary(SupplementaryVolume::try_from(value)?)),
            3 => Ok(Self::Volume(VolumePartition::try_from(value)?)),
            255 => Ok(Self::Terminator(VolumeTerminator::try_from(value)?)),
            _ => Err("Invalid iso9660 volume descriptor".to_string()),
        }
    }
}

pub struct Iso9660Image {
    /// System area, must be 0x8000 bytes
    sys_area: Vec<u8>,
    volume_descriptors: Vec<Iso9660Volume>,
}

impl Iso9660Image {
    /// construct an empty iso image
    pub fn new() -> Self {
        Self {
            sys_area: Vec::new(),
            volume_descriptors: Vec::new(),
        }
    }

    pub fn parse_bytes(b: &[u8]) -> Result<Self, String> {
        let mut s = std::io::BufReader::new(b);
        let mut sys = [0u8; 0x8000];
        s.read_exact(&mut sys).map_err(|e| format!("Invalid sys region: {}", e))?;
        let mut done = false;
        let mut descriptors = Vec::new();
        while !done {
            let mut descriptor = [0u8; 2048];
            s.read_exact(&mut descriptor).map_err(|e| format!("Unable to read volume descriptor: {}", e))?;
            let p = Iso9660Volume::try_from(&descriptor).inspect_err(|e| log::error!("Invalid volume descriptor: {}", e))?;
            if let Iso9660Volume::Terminator(_) = &p {
                done = true;
            }
            descriptors.push(p);
        }
        Ok(Self {
            sys_area: sys.to_vec(),
            volume_descriptors: descriptors,
        })
    }

    /// Read the contents of an iso image from a path
    pub fn read_iso(p: std::path::PathBuf) -> Result<Self, String> {
        let mut f = std::fs::File::open(p).map_err(|e| e.to_string())?;
        let mut contents = Vec::new();
        f.read_to_end(&mut contents).map_err(|e| e.to_string())?;
        Self::parse_bytes(&contents)
    }

    /// Write this iso image to a file
    pub fn write_to_file(&self, s: std::path::PathBuf) -> Result<(), std::io::Error> {
        let mut f = std::fs::File::create(s)?;
        f.write_all(&self.sys_area)?;
        for v in &self.volume_descriptors {
            let mut vb: [u8; 2048] = [0; 2048];
            v.get_contents(&mut vb);
            f.write_all(&vb)?;
        }
        Ok(())
    }
}