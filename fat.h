#include "filesystem.h"	//disk access routines
#ifndef _FAT_H_
#define _FAT_H_

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
//the upper two bits of this field are reserved

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
	char short_name[11];	//not null terminated
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

class fat : public filesystem
{	//this should inherit a class called filesystem
	public:
		fat();
		~fat();
		int mount(disk *operand_drive, unsigned int drive_num);
		int unmount(disk *operand_drive, unsigned int drive_num);
	private:
		FatBootSector sector;
		unsigned int current_directory;	//this is the sector number for the first sector of the directory
		//TODO: implement current directory name
		//TODO: implement filesystem naming method EX: "/hda/1"
		int valid;	//used to determine if this filesystem is usable or not
		unsigned int FirstSectorOfCluster(unsigned long, unsigned long, unsigned long);
		int load_cluster(disk *, unsigned int, unsigned long cluster_number, unsigned char *buffer);
		int load_directory(disk *, unsigned int, unsigned char *buffer);
		int isFat32();
		int isFat16();
		int isFat12();
		int load_boot_sector(disk *, unsigned int, unsigned char *);
		int get_cluster_entry(disk *, unsigned int, unsigned int cluster_num);
		int is_eof(unsigned int cluster_value);
		int is_short_name_valid(char *);	//checks an 11 byte name for valid characters
		
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
