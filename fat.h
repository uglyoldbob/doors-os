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
	unsigned long FirstDataSector;	//start of the data region
	unsigned long DataSectors;		//number of sectors holding data
	unsigned long CountOfClusters;	//this is used to determine the FAT type
};
