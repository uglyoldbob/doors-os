#include "fat.h"
#include "floppy.h"
#include "memory.h"

int load_boot_sector(unsigned int drive_num, struct FatBootSector *sector, unsigned char *store)
{	//interprets data from a buffer where a boot sector is to be preloaded
	//if this function returns -1, then the boot sector is not FAT12/16/32

	unsigned long FatSize;	//used to store the appropriate FAT size variable (16 or 32 bit)
	unsigned long TotalSectors;	//used to store the appropriate number of total sectors (16 or 32 bit version)

	memcopy(sector->OemName, &store[3], 8);
	sector->OemName[8] = 0;
	sector->BytesPerSector = store[12] * 0x100 + store[11];
	switch (sector->BytesPerSector)
	{	//check for invalid values
		case 0x1000: case 0x800: case 0x400: case 0x200:
			break;
		default:
			//invalid number of bytes per sector for a FAT file system
			PrintNumber(sector->BytesPerSector);
			display(" bytes per sector is invalid. Invalid FAT.\n");
			return -1;
			break;
	}
	sector->SectorsPerCluster = store[13];
	switch (sector->SectorsPerCluster)
	{
		case 0x1: case 0x2: case 0x4: case 0x8: case 0x10: case 0x20: case 0x40: case 0x80:
			break;
		default:
			PrintNumber(sector->SectorsPerCluster);
			display(" sectors per cluster is invalid. Invalid FAT.\n");
			return -1;
			break;
	}
	sector->ReservedSectorCount = store[15] * 0x100 + store[14];
	if (sector->ReservedSectorCount == 0)
	{
		display("Reserved sector count cannot be 0. Invalid FAT\n");
		return -1;
	}
	sector->NumberFats = store[16];
	if (sector->NumberFats == 0)
	{
		display("Number of FAT tables cannot be 0. Invalid FAT\n");
		return -1;
	}
	sector->NumberRootEntries = store[18] * 0x100 + store[17];
	sector->SectorCount16 = store[20] * 0x100 + store[19];
	sector->Media = store[21];
	PrintNumber(sector->Media);
	display(" is the media type\n");
	sector->FatSize16 = store[23] * 0x100 + store[22];
	sector->SectorsPerTrack = store[25] * 0x100 + store[24];
	sector->NumberHeads = store[27] * 0x100 + store[26];
	sector->HiddenSectorsCount = store[31] * 0x10000 + store[30] * 0x10000 + store[29] * 0x100 + store[28];
	sector->TotalSectors = store[35] * 0x10000 + store[34] * 0x10000 + store[33] * 0x100 + store[32];

	sector->RootDirSectors = (sector->NumberRootEntries * 32) + sector->BytesPerSector - 1;
	if ((sector->RootDirSectors % sector->BytesPerSector) != 0)
	{	//add 1 to the truncated total
		//display("Remainder existed when calculating RootDirSectors\n");
		sector->RootDirSectors = (sector->RootDirSectors / sector->BytesPerSector) + 1;
	}
	else
	{
		//display("No remainder existed when calculating RootDirSectors\n");
		sector->RootDirSectors = (sector->RootDirSectors / sector->BytesPerSector);
	}
	//RootDirSectors is always 0 on a FAT32 folume
	if (sector->FatSize16 == 0)
		FatSize = sector->FatSize32;
	else
		FatSize = sector->FatSize16;
	sector->FirstDataSector = sector->ReservedSectorCount + (sector->NumberFats * FatSize) + sector->RootDirSectors;
	//this sector number is relative to the first sector of the volume (the boot sector)
	//sector 0 of a volume may not be the first sector of the drive (partitioning)
	if (sector->SectorCount16  == 0)
		TotalSectors = sector->TotalSectors;
	else
		TotalSectors = sector->SectorCount16;
	sector->DataSectors = TotalSectors - 
		(sector->ReservedSectorCount + (sector->NumberFats * FatSize) + sector->RootDirSectors);
	sector->CountOfClusters = sector->DataSectors / sector->SectorsPerCluster;
	if (sector->CountOfClusters < 4085)
	{
		display("FAT12 volume detected\n");
	}
	else if (sector->CountOfClusters < 65525)
	{
		display("FAT16 volume detected\n");
	}
	else
	{
		if ((store[43] * 0x100 + store[42]) != 0)
		{
			display("This is not a FAT file system\n");
			return -1;
		}
		else
			display("FAT32 volume detected\n");
	}
	if (sector->CountOfClusters < 65525)
	{
		display("Loading FAT12/FAT16 information\n");
		//these are FAT12/FAT16 specific slots
		sector->DriveNum = store[36];
		sector->ExtendedBootSig = store[37];
		sector->VolumeId = store[42] * 0x10000 + store[41] * 0x10000 + store[40] * 0x100 + store[39];
		memcopy(sector->VolumeLabel, &store[43], 11);
		sector->VolumeLabel[12] = 0;
		memcopy(sector->FileSysType, &store[54], 8);
		sector->FileSysType[12] = 0;
		//FAT32 information will be zeroed
		sector->FatSize32 = 0;
		sector->ExtFlags = 0;
		sector->FS_Version = 0;
		sector->RootCluster = 0;
		sector->FSInfo = 0;
		sector->BackupBootSector = 0;
	}
	else
	{
		display("Loading FAT32 information\n");
		sector->FatSize32 = store[39] * 0x10000 + store[38] * 0x10000 + store[37] * 0x100 + store[36];
		sector->ExtFlags = store[41] * 0x100 + store[40];
		sector->FS_Version = store[43] * 0x100 + store[42];
		sector->RootCluster = store[47] * 0x10000 + store[46] * 0x10000 + store[45] * 0x100 + store[44];
		sector->FSInfo = store[49] * 0x100 + store[48];
		sector->BackupBootSector = store[51] * 0x100 + store[50];
		sector->DriveNum = store[64];
		sector->ExtendedBootSig = store[66];
		sector->VolumeId = store[70] * 0x10000 + store[69] * 0x10000 + store[68] * 0x100 + store[67];
		memcopy(sector->VolumeLabel, &store[71], 11);
		sector->VolumeLabel[12] = 0;
		memcopy(sector->FileSysType, &store[82], 8);
		sector->FileSysType[12] = 0;
	}

	display("OEM Name:");
	display(sector->OemName);
	display("\n");
	display(sector->VolumeLabel);
	display("\n");
	display(sector->FileSysType);
	display("\n");
}

int isFat12(struct FatBootSector *test)
{
	if (test->CountOfClusters < 4085)
	{
		return 0;	//indicate that it is FAT12
	}
	return -1;
}

int isFat16(struct FatBootSector *test)
{
	if (test->CountOfClusters < 4085)
	{
		return -1;	//not FAT16
	}
	else if (test->CountOfClusters < 65525)
	{
		return 0;	//fat16
	}
	return -1;	//not fat16
}

int isFat32(struct FatBootSector *test)
{
	if (test->CountOfClusters < 4085)
	{
		return -1;	//not fat32
	}
	else if (test->CountOfClusters < 65525)
	{
		return -1;	//not fat32
	}
	else
	{
		return 0;	//FAT32
	}
}

unsigned int FirstSectorOfCluster(unsigned long clusterNum, unsigned long sectorsPerCluster, unsigned long firstDataSector)
{
	return ((clusterNum - 2) * sectorsPerCluster) + firstDataSector;
}


