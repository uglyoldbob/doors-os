use std::io::{Read, Write};

/// One of the classes of strings defined by is09660
#[derive(Debug, Default)]
struct Iso9660StringA(String);

/// One of the classes of strings defined by is09660
#[derive(Debug, Default)]
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
                'A'..='Z' | '0'..='9' | ' ' | '_' => {}
                _ => {
                    return Err(format!("Invalid character '{}'", b));
                }
            }
        }
        Ok(Self(s))
    }
}

#[derive(Debug)]
pub struct BootVolume {
    boot_system_id: Iso9660StringA,
    boot_id: Iso9660StringA,
    custom: Vec<u8>,
}

impl TryFrom<&[u8; 2048]> for BootVolume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        let standard = str::from_utf8(&value[1..6])
            .map_err(|e| format!("Invalid standard in primary volume: {}", e))?
            .to_string();
        assert_eq!(standard, "CD001".to_string());
        assert_eq!(1, value[6]); //version
        let bsi = str::from_utf8(value[7..39].trim_ascii_end())
            .map_err(|e| format!("Invalid char in boot system identifier: {:?}", e))?;
        let bsi = bsi.trim_end_matches(char::from(0));
        let bi = str::from_utf8(value[39..71].trim_ascii_end())
            .map_err(|e| format!("Invalid char in boot identifier: {:?}", e))?;
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
    fn write_descriptor(&self, descriptor: &mut [u8; 2048]) {
        descriptor[0] = 0;
        descriptor[2..7].copy_from_slice("CD001".as_bytes());
        descriptor[7] = 1; //version
    }
}

#[derive(Debug, Default)]
pub struct Iso9660DirectoryRecord {
    /// Length of the record in bytes
    length: u8,
    extended_attribute_length: u8,
    location_lba: u32,
    length_bytes: u32,
    date_time_years: u8,
    date_time_month: u8,
    date_time_day: u8,
    date_time_hour: u8,
    date_time_minute: u8,
    date_time_second: u8,
    date_time_gmt_offset: u8,
    flags: u8,
    file_unit_size: u8,
    interleave_gap_size: u8,
    volume_sequence_number: u16,
    filename_length: u8,
    name: Iso9660StringD,
}

impl Iso9660DirectoryRecord {
    /// Write the contents of Self to the given buffer, returning the actual length used
    fn get_contents(&self, buffer: &mut [u8]) -> usize {
        let mut l = 33 + self.name.0.len();
        if (l % 2) == 0 {
            l += 1;
        }
        buffer[0] = l as u8;
        buffer[1] = self.extended_attribute_length;
        buffer[2..6].copy_from_slice(&self.location_lba.to_le_bytes());
        buffer[6..10].copy_from_slice(&self.location_lba.to_be_bytes());
        buffer[10..14].copy_from_slice(&self.length_bytes.to_le_bytes());
        buffer[14..18].copy_from_slice(&self.length_bytes.to_be_bytes());
        buffer[18] = self.date_time_years;
        buffer[19] = self.date_time_month;
        buffer[20] = self.date_time_day;
        buffer[21] = self.date_time_hour;
        buffer[22] = self.date_time_minute;
        buffer[23] = self.date_time_second;
        buffer[24] = self.date_time_gmt_offset;
        buffer[25] = self.flags;
        buffer[26] = self.file_unit_size;
        buffer[27] = self.interleave_gap_size;
        buffer[28..30].copy_from_slice(&self.volume_sequence_number.to_le_bytes());
        buffer[30..32].copy_from_slice(&self.volume_sequence_number.to_be_bytes());
        buffer[32] = self.filename_length;
        if !self.name.0.is_empty() {
            buffer[33..33 + self.name.0.len()].copy_from_slice(self.name.0.as_bytes());
        }
        l
    }
}

impl TryFrom<&[u8]> for Iso9660DirectoryRecord {
    type Error = String;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 34 {
            return Err(format!(
                "Invalid size {} for Iso9660RootDirectoryRecord",
                value.len()
            ));
        }
        let l = value[32];
        let name = if l == 0 {
            String::new()
        } else {
            let s = str::from_utf8(value[33..33 + l as usize].trim_ascii_end())
                .map_err(|e| format!("Invalid char in filename: {:?}", e))?;
            s.trim_end_matches(char::from(0)).to_string()
        };
        Ok(Self {
            length: value[0],
            extended_attribute_length: value[1],
            location_lba: u32::from_le_bytes(value[2..6].try_into().unwrap()),
            length_bytes: u32::from_le_bytes(value[10..14].try_into().unwrap()),
            date_time_years: value[18],
            date_time_month: value[19],
            date_time_day: value[20],
            date_time_hour: value[21],
            date_time_minute: value[22],
            date_time_second: value[23],
            date_time_gmt_offset: value[24],
            flags: value[25],
            file_unit_size: value[26],
            interleave_gap_size: value[27],
            volume_sequence_number: u16::from_le_bytes(value[28..30].try_into().unwrap()),
            filename_length: l,
            name: name.as_str().try_into()?,
        })
    }
}

#[derive(Debug, Default)]
pub struct Iso9660Datetime {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    fractions_second: u8,
    gmt_offset: u8,
}

impl Iso9660Datetime {
    /// Write the contents of Self to the given buffer
    fn get_contents(&self, buffer: &mut [u8]) {
        buffer[0..4].copy_from_slice(format!("{:<4}", self.year).as_bytes());
        buffer[4..6].copy_from_slice(format!("{:<2}", self.month).as_bytes());
        buffer[6..8].copy_from_slice(format!("{:<2}", self.day).as_bytes());
        buffer[8..10].copy_from_slice(format!("{:<2}", self.hour).as_bytes());
        buffer[10..12].copy_from_slice(format!("{:<2}", self.minute).as_bytes());
        buffer[12..14].copy_from_slice(format!("{:<2}", self.second).as_bytes());
        buffer[14..16].copy_from_slice(format!("{:<2}", self.fractions_second).as_bytes());
        buffer[16] = self.gmt_offset;
    }
}

impl TryFrom<&[u8]> for Iso9660Datetime {
    type Error = String;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 17 {
            return Err(format!("Invalid size {} for Iso9660Datetime", value.len()));
        }
        Ok(Self {
            year: str::from_utf8(&value[0..4])
                .map_err(|e| e.to_string())?
                .parse::<u16>()
                .map_err(|e| e.to_string())?,
            month: str::from_utf8(&value[4..6])
                .map_err(|e| e.to_string())?
                .parse::<u8>()
                .map_err(|e| e.to_string())?,
            day: str::from_utf8(&value[6..8])
                .map_err(|e| e.to_string())?
                .parse::<u8>()
                .map_err(|e| e.to_string())?,
            hour: str::from_utf8(&value[8..10])
                .map_err(|e| e.to_string())?
                .parse::<u8>()
                .map_err(|e| e.to_string())?,
            minute: str::from_utf8(&value[10..12])
                .map_err(|e| e.to_string())?
                .parse::<u8>()
                .map_err(|e| e.to_string())?,
            second: str::from_utf8(&value[12..14])
                .map_err(|e| e.to_string())?
                .parse::<u8>()
                .map_err(|e| e.to_string())?,
            fractions_second: str::from_utf8(&value[14..16])
                .map_err(|e| e.to_string())?
                .parse::<u8>()
                .map_err(|e| e.to_string())?,
            gmt_offset: value[16],
        })
    }
}

struct PathTable {
    data: Vec<u8>,
    /// Is this path table stored in big endian format?
    be: bool,
}

impl PathTable {
    fn iter(&self) -> PathTableIterator<'_> {
        PathTableIterator {
            i: 0,
            data: &self.data,
            be: self.be,
        }
    }
}

struct PathTableIterator<'a> {
    i: usize,
    data: &'a [u8],
    be: bool,
}

impl<'a> Iterator for PathTableIterator<'a> {
    type Item = PathTableEntry;
    fn next(&mut self) -> Option<Self::Item> {
        if self.i < self.data.len() {
            let p = if self.be {
                PathTableEntry::from_be_data(&self.data[self.i..])
            } else {
                PathTableEntry::from_le_data(&self.data[self.i..])
            };
            if let Ok(p) = &p {
                self.i += p.length();
            }
            if let Err(e) = &p {
                log::error!("Error decoding path table entry: {}", e);
            }
            p.ok()
        } else {
            None
        }
    }
}

/// An entry in the path table
#[derive(Debug)]
struct PathTableEntry {
    name_length: u8,
    extended_attribute_length: u8,
    location_lba: u32,
    parent_dir: u16,
    name: Iso9660StringD,
}

impl PathTableEntry {
    /// Calculate the length in bytes
    fn length(&self) -> usize {
        let mut len = 8;
        len += self.name_length as usize;
        if (len % 2) == 1 {
            len += 1;
        }
        len
    }

    fn directory_entry_location(&self) -> usize {
        self.location_lba as usize * 2048
    }

    fn write_data(&self, buf: &mut [u8], be: bool) {
        buf[0] = self.name_length;
        buf[1] = self.extended_attribute_length;
        if be {
            buf[2..6].copy_from_slice(&self.location_lba.to_be_bytes());
            buf[6..8].copy_from_slice(&self.parent_dir.to_be_bytes());
        } else {
            buf[2..6].copy_from_slice(&self.location_lba.to_le_bytes());
            buf[6..8].copy_from_slice(&self.parent_dir.to_le_bytes());
        }
    }

    /// Build a Self from the buffer holding little endian values
    fn from_le_data(buf: &[u8]) -> Result<Self, String> {
        let name_length = buf[0];
        let name = if name_length == 0 {
            String::new()
        } else {
            let s = str::from_utf8(buf[8..8 + name_length as usize].trim_ascii_end())
                .map_err(|e| format!("Invalid char in filename: {:?}", e))?;
            s.trim_end_matches(char::from(0)).to_string()
        };
        Ok(Self {
            name_length,
            extended_attribute_length: buf[1],
            location_lba: u32::from_le_bytes(buf[2..6].try_into().unwrap()),
            parent_dir: u16::from_le_bytes(buf[6..8].try_into().unwrap()),
            name: name
                .as_str()
                .try_into()
                .map_err(|e| format!("Invalid char in filename {}", e))?,
        })
    }

    /// Build a Self from the buffer holding big endian values
    fn from_be_data(buf: &[u8]) -> Result<Self, String> {
        let name_length = buf[0];
        let name = if name_length == 0 {
            String::new()
        } else {
            let s = str::from_utf8(buf[8..8 + name_length as usize].trim_ascii_end())
                .map_err(|e| format!("Invalid char in filename: {:?}", e))?;
            s.trim_end_matches(char::from(0)).to_string()
        };
        Ok(Self {
            name_length,
            extended_attribute_length: buf[1],
            location_lba: u32::from_be_bytes(buf[2..6].try_into().unwrap()),
            parent_dir: u16::from_be_bytes(buf[6..8].try_into().unwrap()),
            name: name
                .as_str()
                .try_into()
                .map_err(|e| format!("Invalid char in filename {}", e))?,
        })
    }
}

#[derive(Debug)]
pub struct PrimaryVolume {
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
    /// block size in bytes
    block_size: u16,
    /// path table size in bytes
    path_table_size: u32,
    /// Path table location in lba form
    path_table_location: u32,
    /// optional path table location in lba form
    optional_path_table_location: u32,
    /// The root directory
    root: Iso9660DirectoryRecord,
    /// volume set id
    volume_set_id: Iso9660StringD,
    /// publisher id
    publisher: Iso9660StringA,
    /// The id of the person that prepared the data
    data_preparer: Iso9660StringA,
    /// application id
    application: Iso9660StringA,
    /// copyright filename, the file that contains the copyright information
    copyright_file: Iso9660StringD,
    /// abstract filename, the file that contains abstract information for the volume
    abstract_file: Iso9660StringD,
    /// bibliographic filename, the file containint bibliographic information
    bibliographic_file: Iso9660StringD,
    /// creation date and time
    creation: Iso9660Datetime,
    /// modification date and time
    modification: Iso9660Datetime,
    /// when the volume data is considered to be obsolete
    expiration: Iso9660Datetime,
    /// When the volume data starts being useful.
    beginning: Iso9660Datetime,
    /// application specific data
    application_specific: [u8; 512],
    /// Path table
    path_table: Vec<PathTableEntry>,
    /// The volume data
    volume_data: Vec<u8>,
}

impl TryFrom<&[u8; 2048]> for PrimaryVolume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        let standard = str::from_utf8(&value[1..6])
            .map_err(|e| format!("Invalid standard in primary volume: {}", e))?
            .to_string();
        assert_eq!(standard, "CD001".to_string());
        assert_eq!(1, value[6]); //version
        assert_eq!(0, value[7]); //unused
        let sys = str::from_utf8(value[8..40].trim_ascii_end())
            .map_err(|e| format!("Invalid char in system identifier: {:?}", e))?;
        let vol = str::from_utf8(value[40..72].trim_ascii_end())
            .map_err(|e| format!("Invalid char in volume identifier: {:?}", e))?;
        log::info!("Vol is {}", vol);
        Ok(Self {
            system: sys.try_into()?,
            volume_id: vol.try_into()?,
            size: u32::from_le_bytes(value[80..84].try_into().unwrap()),
            total_volumes: u16::from_le_bytes(value[120..122].try_into().unwrap()),
            this_volume: u16::from_le_bytes(value[124..126].try_into().unwrap()),
            block_size: u16::from_le_bytes(value[128..130].try_into().unwrap()),
            path_table_size: u32::from_le_bytes(value[132..136].try_into().unwrap()),
            path_table_location: u32::from_le_bytes(value[140..144].try_into().unwrap()),
            optional_path_table_location: u32::from_le_bytes(value[144..148].try_into().unwrap()),
            root: (&value[156..190])
                .try_into()
                .map_err(|e| format!("Invalid root directory record: {:?}", e))?,
            volume_set_id: str::from_utf8(value[190..318].trim_ascii_end())
                .map_err(|e| format!("Invalid char in volume set id: {:?}", e))?
                .try_into()?,
            publisher: str::from_utf8(value[318..446].trim_ascii_end())
                .map_err(|e| format!("Invalid char in publisher id: {:?}", e))?
                .try_into()?,
            data_preparer: str::from_utf8(value[446..574].trim_ascii_end())
                .map_err(|e| format!("Invalid char in data preparer id: {:?}", e))?
                .try_into()?,
            application: str::from_utf8(value[574..702].trim_ascii_end())
                .map_err(|e| format!("Invalid char in application id: {:?}", e))?
                .try_into()?,
            copyright_file: str::from_utf8(value[702..739].trim_ascii_end())
                .map_err(|e| format!("Invalid char in copyright filename: {:?}", e))?
                .try_into()?,
            abstract_file: str::from_utf8(value[739..776].trim_ascii_end())
                .map_err(|e| format!("Invalid char in abstract filename: {:?}", e))?
                .try_into()?,
            bibliographic_file: str::from_utf8(value[776..813].trim_ascii_end())
                .map_err(|e| format!("Invalid char in bibliographic filename: {:?}", e))?
                .try_into()?,
            creation: (&value[813..830])
                .try_into()
                .map_err(|e| format!("Invalid creation date time: {:?}", e))?,
            modification: (&value[830..847])
                .try_into()
                .map_err(|e| format!("Invalid modification date time: {:?}", e))?,
            expiration: (&value[847..864])
                .try_into()
                .map_err(|e| format!("Invalid creation date time: {:?}", e))?,
            beginning: (&value[864..881])
                .try_into()
                .map_err(|e| format!("Invalid creation date time: {:?}", e))?,
            application_specific: value[883..1395].try_into().unwrap(),
            path_table: Vec::new(),
            volume_data: Vec::new(),
        })
    }
}

impl Default for PrimaryVolume {
    fn default() -> Self {
        Self {
            system: " ".repeat(32).as_str().try_into().unwrap(),
            volume_id: "DEFAULT".try_into().unwrap(),
            size: 1,
            total_volumes: 1,
            this_volume: 1,
            block_size: 2048,
            path_table_size: 1,
            path_table_location: 0,
            optional_path_table_location: 0,
            root: Iso9660DirectoryRecord::default(),
            volume_set_id: Default::default(),
            publisher: Default::default(),
            data_preparer: Default::default(),
            application: Default::default(),
            copyright_file: Default::default(),
            abstract_file: Default::default(),
            bibliographic_file: Default::default(),
            creation: Default::default(),
            modification: Default::default(),
            expiration: Default::default(),
            beginning: Default::default(),
            application_specific: [0; 512],
            path_table: Vec::new(),
            volume_data: Vec::new(),
        }
    }
}

impl Iso9660VolumeTrait for PrimaryVolume {
    fn write_descriptor(&self, descriptor: &mut [u8; 2048]) {
        descriptor[0] = 1;
        descriptor[2..7].copy_from_slice("CD001".as_bytes());
        descriptor[7] = 1; //version
        descriptor[8..40].copy_from_slice(format!("{:<32}", self.system.0).as_bytes());
        descriptor[40..72].copy_from_slice(format!("{:<32}", self.volume_id.0).as_bytes());
        descriptor[72..80].copy_from_slice(&[0; 8]);
        descriptor[80..84].copy_from_slice(&self.size.to_le_bytes());
        descriptor[84..88].copy_from_slice(&self.size.to_be_bytes());
        descriptor[88..120].copy_from_slice(&[0; 32]);
        descriptor[120..122].copy_from_slice(&self.total_volumes.to_le_bytes());
        descriptor[122..124].copy_from_slice(&self.total_volumes.to_be_bytes());
        descriptor[124..126].copy_from_slice(&self.this_volume.to_le_bytes());
        descriptor[126..128].copy_from_slice(&self.this_volume.to_be_bytes());
        descriptor[128..130].copy_from_slice(&self.block_size.to_le_bytes());
        descriptor[130..132].copy_from_slice(&self.block_size.to_be_bytes());
        descriptor[132..136].copy_from_slice(&self.path_table_size.to_le_bytes());
        descriptor[136..140].copy_from_slice(&self.path_table_size.to_be_bytes());
        descriptor[140..144].copy_from_slice(&self.path_table_location.to_le_bytes());
        descriptor[144..148].copy_from_slice(&self.optional_path_table_location.to_le_bytes());
        descriptor[148..152].copy_from_slice(&self.path_table_location.to_be_bytes());
        descriptor[152..156].copy_from_slice(&self.optional_path_table_location.to_be_bytes());
        self.root.get_contents(&mut descriptor[156..190]);
        descriptor[190..318].copy_from_slice(format!("{:<128}", self.volume_set_id.0).as_bytes());
        descriptor[318..446].copy_from_slice(format!("{:<128}", self.publisher.0).as_bytes());
        descriptor[446..574].copy_from_slice(format!("{:<128}", self.data_preparer.0).as_bytes());
        descriptor[574..702].copy_from_slice(format!("{:<128}", self.application.0).as_bytes());
        descriptor[702..739].copy_from_slice(format!("{:<37}", self.copyright_file.0).as_bytes());
        descriptor[739..776].copy_from_slice(format!("{:<37}", self.abstract_file.0).as_bytes());
        descriptor[776..813]
            .copy_from_slice(format!("{:<37}", self.bibliographic_file.0).as_bytes());
        self.creation.get_contents(&mut descriptor[813..830]);
        self.modification.get_contents(&mut descriptor[830..847]);
        self.expiration.get_contents(&mut descriptor[847..864]);
        self.beginning.get_contents(&mut descriptor[864..881]);
        descriptor[881] = 1; //file structure version
        descriptor[882] = 0;
        descriptor[883..1395].copy_from_slice(&self.application_specific);
    }

    fn read_volume_data(&mut self, iso: &[u8]) {
        let pt_start = self.path_table_location as usize * 2048;
        let pt_size = self.path_table_size as usize;
        let path_table_data = iso[pt_start..pt_start + pt_size].to_vec();
        let pt = PathTable {
            data: path_table_data,
            be: false,
        };
        let pt: Vec<PathTableEntry> = pt.iter().collect();
        log::info!("Showing path table entries");
        for i in pt.iter() {
            log::info!("Path table entry: len {} {:02x?}", i.length(), i);
            let lba = i.directory_entry_location();
            let size = iso[lba];
            log::info!("Directory size is {} at {:x}", size, lba);
            let dir_contents = &iso[lba..lba + size as usize];
            let d: Iso9660DirectoryRecord = dir_contents.try_into().unwrap();
            log::info!("Directory record is {:?}", d);
        }
        self.path_table = pt;
    }
}

#[derive(Debug)]
pub struct SupplementaryVolume {}

impl TryFrom<&[u8; 2048]> for SupplementaryVolume {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        todo!();
        Ok(Self {})
    }
}

impl Iso9660VolumeTrait for SupplementaryVolume {
    fn write_descriptor(&self, descriptor: &mut [u8; 2048]) {}
}

#[derive(Debug)]
pub struct VolumePartition {}

impl TryFrom<&[u8; 2048]> for VolumePartition {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        todo!();
        Ok(Self {})
    }
}

impl Iso9660VolumeTrait for VolumePartition {
    fn write_descriptor(&self, descriptor: &mut [u8; 2048]) {}
}

#[derive(Debug)]
pub struct VolumeTerminator {}

impl TryFrom<&[u8; 2048]> for VolumeTerminator {
    type Error = String;
    fn try_from(value: &[u8; 2048]) -> Result<Self, Self::Error> {
        let standard = str::from_utf8(&value[1..6])
            .map_err(|e| format!("Invalid standard in primary volume: {}", e))?
            .to_string();
        assert_eq!(standard, "CD001".to_string());
        assert_eq!(1, value[6]); //version
        Ok(Self {})
    }
}

impl Iso9660VolumeTrait for VolumeTerminator {
    fn write_descriptor(&self, descriptor: &mut [u8; 2048]) {
        descriptor[0] = 255;
        descriptor[1..6].copy_from_slice("CD001".as_bytes());
        descriptor[6] = 1;
    }
}

/// The volume trait for volumes in an iso9660 image
#[enum_dispatch::enum_dispatch]
trait Iso9660VolumeTrait {
    /// Write the contents of the volume descriptor to the specified buffer
    fn write_descriptor(&self, descriptor: &mut [u8; 2048]);
    /// Read the volume contents if applicable, storing them as necessary
    fn read_volume_data(&mut self, _iso: &[u8]) {}
    /// Get the contents of the volume, if applicable
    fn get_volume_data(&self) -> Option<Vec<u8>> {
        None
    }
}

#[derive(Debug)]
#[enum_dispatch::enum_dispatch(Iso9660VolumeTrait)]
pub enum Iso9660Volume {
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
        s.read_exact(&mut sys)
            .map_err(|e| format!("Invalid sys region: {}", e))?;
        let mut done = false;
        let mut descriptors = Vec::new();
        while !done {
            let mut descriptor = [0u8; 2048];
            s.read_exact(&mut descriptor)
                .map_err(|e| format!("Unable to read volume descriptor: {}", e))?;
            let mut p = Iso9660Volume::try_from(&descriptor)
                .inspect_err(|e| log::error!("Invalid volume descriptor: {}", e))?;
            p.read_volume_data(b);
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

    /// Iterate over all volume descriptors
    pub fn volume_descriptors(&self) -> impl Iterator<Item = &Iso9660Volume> {
        self.volume_descriptors.iter()
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
            v.write_descriptor(&mut vb);
            f.write_all(&vb)?;
        }
        Ok(())
    }
}
