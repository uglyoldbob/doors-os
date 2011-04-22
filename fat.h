#ifndef _FAT_H_
#define _FAT_H_
#include "filesystem.h"	//disk access routines
#include "disk.h"

struct FatBootSector
{
	char OemName[9];	//usually "MSWIN4.1"
	unsigned long BytesPerSector;	//usually 0x200
	unsigned char SectorsPerCluster;
	unsigned short ReservedSectorCount;	//number of reserved sectors starting with the first sector (usually 1)
	unsigned char NumberFats;	//number of file allocation tables
	unsigned short NumberRootEntries;	//0x200 for FAT16
	unsigned short SectorCount16;	//16 bit field for number of sectors
	unsigned char Media;	//dates back to DOS 1.x, not normally used anymore
	unsigned short FatSize16;		//16 bit field for the size of the fat in sectors
	unsigned short SectorsPerTrack;
	unsigned short NumberHeads;
	unsigned long HiddenSectorsCount;	//number of hidden sectors before the partition containing this fat volume
	unsigned long TotalSectors;	//32 bit field for number of sectors
	//these are FAT12/FAT16 specific
	unsigned char DriveNum;	//drive number assigned by int 0x13 (OS specific) (ex: 0 is floppy, 0x80 is hard drive)
	unsigned char ExtendedBootSig;	//indicates the presence of the next three fields with the value 0x29
	unsigned long VolumeId;	//serial number (for determining the presence of the correct removable disk) (usually date and time together)
	char VolumeLabel[12];	//matches the entry in the root directory
	char FileSysType[9];	//not supposed to be used to determine fat type
	//everything below this point is for FAT32 only
	unsigned long FatSize32;	//number of sectors occupied by one FAT
	unsigned short ExtFlags;	//used for mirroring
	unsigned short FS_Version;	//used to identify which version of FAT32 this is (0:0 is the current version)
	unsigned long RootCluster;	//the cluster number of the first cluster of the root directory
	unsigned short FSInfo;	//sector number of the FSInfo structure
	unsigned short BackupBootSector;	//sector number in the reserved area where a copy of the boot sector	

	//calculated numbers
	unsigned long RootDirSectors;	//sectors taken up by the root directory (0 for FAT32)
	unsigned long FirstRootDirSector;	//first sector of the root directory
	unsigned long FirstDataSector;	//start of the data region
	unsigned long DataSectors;		//number of sectors holding data
	unsigned long CountOfClusters;	//this is used to determine the FAT type
	
};

//these are for the attribute flags in the structure defined below
#define ATTR_READ_ONLY	0x01
#define ATTR_HIDDEN			0x02
#define ATTR_SYSTEM			0x04
#define ATTR_VOLUME_ID	0x08
#define ATTR_DIRECTORY	0x10
#define ATTR_ARCHIVE		0x20
#define ATTR_LONG_NAME	ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID
#define ATTR_LONG_NAME_MASK ATTR_LONG_NAME | ATTR_DIRECTORY | ATTR_DIRECTORY
//the upper two bits of this field are reserved

//TODO: read up on long filenames and work on some code to read entries from a directory
	//then work on changing directories

//these defines are used in reference to the first byte of the name field
#define DIR_FREE				0xE5
#define DIR_AFTER_LAST	0x00	//this entry is free, and it is the last entry
#define DIR_SPECIAL			0x05	//this is actually an 0xE5 byte for the first byte (Japanese character set compatibility)

//date format
//bits 0-4  - day of month
//bits 5-8  - month of year
//fits 9-15 - years after 1980

//time format
//bits 0-4   - 2 second count (0-29)
//bits 5-10  - minutes count (0-59)
//bits 11-23 - hours count (0-23)


struct directory_entry
{
	unsigned char short_name[11];	//not null terminated
	unsigned char attribute;	//file attributes
	unsigned char nt_flag;
	unsigned char time_tenth;	//time of creation in milliseconds (0-199)
	unsigned short time_creation;
	unsigned short date_creation;
	unsigned short access_date;
	unsigned short first_cluster_high;
	unsigned short write_time;	//time of last write, to include file creation
	unsigned short write_date;	//date of last write, to include file creation
	unsigned short first_cluster_lo;
	unsigned int file_size;
} __attribute__((packed));

struct FAT32_FS_info
{
	unsigned long signature;	//0x41615252
	unsigned char reserved1[480];	//reserved for expansion (initialize to 0)
	unsigned long signature2;	//0x61417272
	unsigned long free_count;	//last known number of free clusters (0xFFFFFFFF means unknown)
	unsigned long next_free;	//hint for the driver, it points to a free cluster (again -1 means unknown)
	unsigned char reserved2[12];	//more reserved data (initialize to 0)
	unsigned long signature3;	//0xAA550000	
} __attribute__((packed));

struct long_directory
{
	unsigned char order;	//order of this entry in the sequence of long filename entries
		//0x40 is the last entry, 
	unsigned short name[5];	//first 5 characters of the name
	unsigned char attr;	//attributes - ATTR_LONG_NAME
	unsigned char type;	//0 - directory entry that is a subcomponent of a long filename
	unsigned char checksum;	//checksum of the short name at the end of the long dir set
	unsigned short name2[6];	//letters 6-11 of the long name
	unsigned short firstCluster;	//must be 0, this is the artificial first cluster
	unsigned short name3[2];	//letters 12-13 of the long name
} __attribute__((packed));

//Nth long name entry (n | 0x40)
//... more entries
//first long name entry
//short name entry
//if the checksum found in the long entries and the checksum calculated from the short name do not match
	//then the long name entries are declared as an orphan and are not usable
//names are "null padded" with 0xFFFF in order to detect corruption of long name fields by retarded disk utilities
//short names are always uppercase
//long names use unicode (16 bit) characters
//FAT is case insensitive
//short name search only searches short names
//long name search searches both long and short names
//unicode characters that cannot be represented are represented with a _
//directories obviously cannot have the same name as a file
//the long name of a directory is given
	//the short name is calculated from the long name given

//short name generation from long filenames
//the short name is called basis-name and optional numeric tail
	//convert to upper case
	//convert to OEM
	//if the character does not exist on OEM, replace with _, or set a lossy conversion flag
	//strip leading and embedded spaces
	//strip leading periods
	//while (not at the end) and (character is not a period) and (characters is < 8)
		//copy characters into the short name
	//Insert a dot at the end of the primary components of the basis-name iff the basis name has an
		//extension after the last period in the name.
	//what the heck? (above)
	//scan for the last embedded period in the long filename
		//if found, copy three or less characters into the extension
//numeric tail generation
	//if the conversion is not lossy
	//and the name fits
	//and the name is not a duplicate
	//no numeric tail is required
	//if the name is a duplicate
	//insert a "~n" to the end of the name
	

//validating directory contents
	//reserved fields can have non-zero values, that is fine
	//do not zero out non-zero reserved fields
	
/* Use this to determine if a directory entry is a long or short entry
if (((LDIR_attr & ATTR_LONG_NAME_MASK) == ATTR_LONG_NAME) && (LDIR_Ord != 0xE5))
{
      //* Found an active long name sub-component.
}
*/

/* use this to determine the type of short entry being inspected
if (((LDIR_attr & ATTR_LONG_NAME_MASK) != ATTR_LONG_NAME) && (LDIR_Ord != 0xE5))
{
      if      ((DIR_Attr & (ATTR_DIRECTORY | ATTR_VOLUME_ID)) == 0x00)
                 // Found a file. 
      else if ((DIR_Attr & (ATTR_DIRECTORY | ATTR_VOLUME_ID)) == ATTR_DIRECTORY)
                 // Found a directory. 
      else if ((DIR_Attr & (ATTR_DIRECTORY | ATTR_VOLUME_ID)) == ATTR_VOLUME_ID)
                 // Found a volume label. 
      else
                 // Found an invalid directory entry. 
}*/
//non-zero values in the type field does not mean that it is invalid
//use the checksum to verify the long name and short name mesh together



class fat : public filesystem
{	//this should inherit a class called filesystem
	public:
		fat();
		~fat();
		int mount(disk *operand_drive, unsigned int drive_num);
		int unmount(disk *operand_drive, unsigned int drive_num);
		dir_item *list_contents();
		int enter_directory(const char* dir);	//enters the directory given
			//. and .. will be processed accordingly unless it is the root directory
			//if it is the root directory, those directories will not be found
		krnl_FILE *open_file(char *filename);
		unsigned int eof(krnl_FILE *descriptor);
		unsigned long seek(krnl_FILE *descriptor, unsigned long position, filesystem *owner);
		unsigned char get_b(krnl_FILE *descriptor, filesystem *owner);
		unsigned short get_w(krnl_FILE *descriptor, filesystem *owner);
		unsigned long get_dw(krnl_FILE *descriptor, filesystem *owner);
		unsigned int get_buffer_size();
			//returns the size of the block that file data is passed in
		unsigned char *get_buffer(unsigned int offset, krnl_FILE *file);
			//returns the buffer that contains the proper offset into the file
	private:
		FatBootSector sector;
		disk *fat_disk;
		unsigned int drive_num;
		int depth;	//stores the folder depth (1 is the root directory)
		unsigned int current_directory;	//this is the number of the first cluster of the directory
		//TODO: implement filesystem naming method EX: "/hda/1"
		int valid;	//used to determine if this filesystem is usable or not
		unsigned int FirstSectorOfCluster(unsigned long, unsigned long, unsigned long);
		int load_cluster(disk *, unsigned int, unsigned long cluster_number, unsigned char *buffer);
		int isFat32();
		int isFat16();
		int isFat12();
		int load_boot_sector(disk *, unsigned int, unsigned char *);
		int get_cluster_entry(disk *, unsigned int, unsigned int cluster_num);
		int is_eof(unsigned int cluster_value);
			//0 means yes, -1 means no
		int is_bad_cluster(unsigned int cluster_value);
		int is_short_name_valid(char *);	//checks an 11 byte name for valid characters
		unsigned int long_name_checksum(unsigned char *pFcbName);
		int is_mounted();
			//-1 is not mounted, 1 is mounted
		unsigned short *extract_filename(long_directory *entry);
			//extracts the filename from a long filename entry
		unsigned long find_dir(const char* directory);
			//attempts to find the first cluster for the directory given
			//returns EOC/EOF if it is unable to find the directory
		dir_item *find_file(const char* filename);
			//attempts to find the first cluster for the filename given
			//returns EOC/EOF if it is unable to find the filename
		dir_item prepare_entry(directory_entry &sample);
			//loads a dir_item structure based on information available from the fat specific data structure
		//load cluster
		//is cluster bad? function
		
};

#endif


//a directory is a file composed of an array of 32 byte structures
//special directory is the root directory


//ROOT DIRECTORY
//no date or time stamps
//no name besides implied '\'
//no '.' or '..' entry
//only directory that can validly contain the ATTR_VOLUME_ID flag
//fat12/16 have a fixed size and location root directory
//fat32 has a fixed location root directory cluster chain root directory
	//the fat32 root directory is like a normal directory

//free clusters are clusters with a value of 0 in their cluster entry
//number of free clusters is not stored anywhere on the volume
//fat32 fsinfo might hold a valid count of free clusters for the volume

//two reserved cluster at the beginning of the FAT
	//first cluster contains BPB_media in the low 8 bits
	//everything else is held high
//second cluster of the FAT is the eoc mark
	//fat16/32 (high 2 bits of this can be used for dirty volume flags
	//0x8000 - clean shut fat16; 0x08000000 - clean shut fat32
	//0x4000 - hard error fat16; 0x04000000 - hard error fat32
//clean shut, 1 = volume clean, 0 = volume is dirty
	//if the volume is dirty then the volume was not dismounted properly
	//check file system integrity
//hard error, 1 = no errors, 0 = disk I/O error last time disk was mounted
	//check for bad sectors and check file system integrity
//FAT stops at countofclusters+1
//the fat size declared and the count of clusters may mean that there are unused sectors at the end of the FAT
	//last sector of the fat is determined by countofclusters+1 instead of using fatsize
//formatting will not be included in the read-only driver for space-saving reasons
	//this may change at a later date

//fat32 volumes have a backup bootsector on sector 6 (count from sector 0)
	//it is actually 3 sectors long containing first sector 0, fsinfo sector, then the last is ????
