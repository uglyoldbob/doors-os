//fat.cpp
//reads from FAT12/16/32
#include "disk.h"
#include "fat.h"
#include "memory.h"
#include "video.h"
#include "entrance.h"

fat::fat()
{	//initialize so that nothing will work	
	valid = -1;
	depth = 1;
}

int fat::load_boot_sector(disk *somewhere, unsigned int drive, unsigned char *store)
{	//interprets data from a buffer where a boot sector is to be preloaded
	//if this function returns -1, then the boot sector is not FAT12/16/32
	fat_disk = somewhere;
	drive_num = drive;
	unsigned long FatSize;	//used to store the appropriate FAT size variable (16 or 32 bit)
	unsigned long TotalSectors;	//used to store the appropriate number of total sectors (16 or 32 bit version)

	if (somewhere->read_sector(drive, 0, (unsigned int*)store) == -1)
		return -1;
	memcopy(sector.OemName, &store[3], 8);
	sector.OemName[7] = 0;
	sector.BytesPerSector = store[12] * 0x100 + store[11];
	switch (sector.BytesPerSector)
	{	//check for invalid values
		case 0x1000: case 0x800: case 0x400: case 0x200:
			break;
		default:
			//invalid number of bytes per sector for a FAT file system
			PrintNumber(sector.BytesPerSector);
			display(" bytes per sector is invalid. Invalid FAT.\n");
			return -1;
			break;
	}
	sector.SectorsPerCluster = store[13];
	switch (sector.SectorsPerCluster)
	{
		case 0x1: case 0x2: case 0x4: case 0x8: case 0x10: case 0x20: case 0x40: case 0x80:
			break;
		default:
			PrintNumber(sector.SectorsPerCluster);
			display(" sectors per cluster is invalid. Invalid FAT.\n");
			return -1;
			break;
	}
	sector.ReservedSectorCount = store[15] * 0x100 + store[14];
	if (sector.ReservedSectorCount == 0)
	{
		display("Reserved sector count cannot be 0. Invalid FAT\n");
		return -1;
	}
	sector.NumberFats = store[16];
	if (sector.NumberFats == 0)
	{
		display("Number of FAT tables cannot be 0. Invalid FAT\n");
		return -1;
	}
	sector.NumberRootEntries = store[18] * 0x100 + store[17];
	sector.SectorCount16 = store[20] * 0x100 + store[19];
	sector.Media = store[21];
	PrintNumber(sector.Media);
	display(" is the media type\n");
	sector.FatSize16 = store[23] * 0x100 + store[22];
	sector.SectorsPerTrack = store[25] * 0x100 + store[24];
	sector.NumberHeads = store[27] * 0x100 + store[26];
	sector.HiddenSectorsCount = store[31] * 0x10000 + store[30] * 0x10000 + store[29] * 0x100 + store[28];
	sector.TotalSectors = store[35] * 0x10000 + store[34] * 0x10000 + store[33] * 0x100 + store[32];

	sector.RootDirSectors = (sector.NumberRootEntries * 32) + sector.BytesPerSector - 1;
	if ((sector.RootDirSectors % sector.BytesPerSector) != 0)
	{	//add 1 to the truncated total
		display("Remainder existed when calculating RootDirSectors\n");
		sector.RootDirSectors = (sector.RootDirSectors / sector.BytesPerSector) + 1;
	}
	else
	{
		display("No remainder existed when calculating RootDirSectors\n");
		sector.RootDirSectors = (sector.RootDirSectors / sector.BytesPerSector);
		PrintNumber(sector.RootDirSectors);
		display("\n");
	}
	//RootDirSectors is always 0 on a FAT32 folume
	if (sector.FatSize16 == 0)
		FatSize = sector.FatSize32;
	else
		FatSize = sector.FatSize16;
	sector.FirstDataSector = sector.ReservedSectorCount + (sector.NumberFats * FatSize) + sector.RootDirSectors - 1;
		//- 1?
	display("FirstDataSector: ");
	PrintNumber(sector.FirstDataSector);
	display("\n");
	//this sector number is relative to the first sector of the volume (the boot sector)
	//sector 0 of a volume may not be the first sector of the drive (partitioning)
	if (sector.SectorCount16  == 0)
		TotalSectors = sector.TotalSectors;
	else
		TotalSectors = sector.SectorCount16;
	sector.DataSectors = TotalSectors - 
		(sector.ReservedSectorCount + (sector.NumberFats * FatSize) + sector.RootDirSectors);
	sector.CountOfClusters = sector.DataSectors / sector.SectorsPerCluster;
	if (sector.CountOfClusters < 4085)
	{
		display("FAT12 volume detected\n");
	}
	else if (sector.CountOfClusters < 65525)
	{
		display("FAT16 volume detected\n");
	}
	else
	{
		if ((store[43] * 0x100 + store[42]) != 0)
		{
			display("This is not a recognizable FAT file system\n");
			return -1;
		}
		else
			display("FAT32 volume detected\n");
	}
	if (sector.CountOfClusters < 65525)
	{
		display("Loading FAT12/FAT16 information\n");
		//these are FAT12/FAT16 specific slots
		current_directory = 0;	//FAT12/16 have a different root directory layout
		sector.DriveNum = store[36];
		sector.ExtendedBootSig = store[37];
		sector.VolumeId = store[42] * 0x10000 + store[41] * 0x10000 + store[40] * 0x100 + store[39];
		memcopy(sector.VolumeLabel, &store[43], 11);
		sector.VolumeLabel[11] = 0;
		memcopy(sector.FileSysType, &store[54], 8);
		sector.FileSysType[8] = 0;
		sector.FirstRootDirSector = sector.ReservedSectorCount + (sector.NumberFats * FatSize);
			//TODO: verify this calculation (make sure that the first sector is now named sector 1
		current_directory = 0;
		depth = 1;
		//FAT32 information will be zeroed
		sector.FatSize32 = 0;
		sector.ExtFlags = 0;
		sector.FS_Version = 0;
		sector.RootCluster = 0;
		sector.FSInfo = 0;
		sector.BackupBootSector = 0;
	}
	else
	{
		display("Loading FAT32 information\n");
		sector.FatSize32 = store[39] * 0x10000 + store[38] * 0x10000 + store[37] * 0x100 + store[36];
		sector.ExtFlags = store[41] * 0x100 + store[40];
		sector.FS_Version = store[43] * 0x100 + store[42];
		sector.RootCluster = store[47] * 0x10000 + store[46] * 0x10000 + store[45] * 0x100 + store[44];
		sector.FSInfo = store[49] * 0x100 + store[48];
		sector.BackupBootSector = store[51] * 0x100 + store[50];
		sector.DriveNum = store[64];
		sector.ExtendedBootSig = store[66];
		sector.VolumeId = store[70] * 0x10000 + store[69] * 0x10000 + store[68] * 0x100 + store[67];
		memcopy(sector.VolumeLabel, &store[71], 11);
		sector.VolumeLabel[11] = 0;
		memcopy(sector.FileSysType, &store[82], 8);
		sector.FileSysType[11] = 0;
		sector.FirstRootDirSector = sector.RootCluster;
		current_directory = sector.RootCluster;
		depth = 1;
	}

	display("OEM Name:");
	display(sector.OemName);
	display("\nLabel:");
	display(sector.VolumeLabel);
	display("\nFile System Type:");
	display(sector.FileSysType);
	display("\nsizeof(directory_entry):");
	PrintNumber(sizeof(directory_entry));
	display("\n");
	return 0;
}

fat::~fat()
{	
}

int fat::isFat12()
{
	if (sector.CountOfClusters < 4085)
	{
		return 0;	//indicate that it is FAT12
	}
	return -1;
}

int fat::isFat16()
{
	if (sector.CountOfClusters < 4085)
	{
		return -1;	//not FAT16
	}
	else if (sector.CountOfClusters < 65525)
	{
		return 0;	//fat16
	}
	return -1;	//not fat16
}

int fat::isFat32()
{
	if (sector.CountOfClusters < 4085)
	{
		return -1;	//not fat32
	}
	else if (sector.CountOfClusters < 65525)
	{
		return -1;	//not fat32
	}
	else
	{
		return 0;	//FAT32
	}
}

unsigned int fat::FirstSectorOfCluster(unsigned long clusterNum, unsigned long sectorsPerCluster, unsigned long firstDataSector)
{
	return (((clusterNum - 2) * sectorsPerCluster) + firstDataSector);
}

int fat::mount(disk *somewhere, unsigned int drive_num)
{	//the mount will retrieve all information required to read files and whatnot
	//in other words, this is the manual constructor for the filesystem driver
	fat_disk = somewhere;
	this->drive_num = drive_num;
	if (valid != 1)
	{
		unsigned char *temp;
		temp = (unsigned char*)kmalloc(somewhere->bytes_per_sector(drive_num));
		//need a way to find the size of a sector
		if (load_boot_sector(somewhere, drive_num, temp) == -1)
		{
			display("Mount failed\n");
			return -1;
		}
		valid = 1;
		kfree(temp);
		return 0;
	}
	display ("Already mounted\n");
	return -1;
}

int fat::unmount(disk *somewhere, unsigned int drive_num)
{
	if (valid != 1)
	{
		display("Not mounted, cannot unmount\n");
		return -1;
	}
	valid = 0;
	return 0;
}

dir_item *fat::list_contents()
{	//lists the contents of the current directory
	//if it is a root directory, check to see if it is a FAT12/16 or a FAT32 root directory
	//for FAT12/16, find the first root directory sector and the size of the root directory
	//for FAT32, just find the first cluster for the root directory and read it just like a normal
	unsigned int cluster_count;
	unsigned int cluster_check;

	unsigned int sector_number = sector.FirstRootDirSector;
	unsigned int sector_offset = 0;	

	directory_entry *temp;	//holds the directory 1 cluster/sector at a time (fat specific information)
	dir_item *stuff;	//this will be the eventual return value

	unsigned short *long_name;
	unsigned short *lengthen;	//this is used to lengthed the longname
	unsigned int long_check;	//this stores the checksums for long-name validation

	unsigned int final_entry = 0;

	int keep_going = 1;
	if (valid == -1)
		return 0;
	//this is a normal directory consisting of a chain of clusters
	if (current_directory != 0)
	{		
		cluster_check = current_directory;
		for (cluster_count = 1; 
			is_eof(get_cluster_entry(fat_disk, drive_num, cluster_check)) != 0;
			 cluster_count++)
		{
			cluster_check = get_cluster_entry(fat_disk, drive_num, cluster_check);			
		}
//		display("Test2...\n");
		cluster_check = current_directory;
		//(cluster size * cluster_count) / 32 is the maximum possible entries for the directory
		stuff = new dir_item[((sector.BytesPerSector * sector.SectorsPerCluster * cluster_count) / 32)];
		//temp uses the fat-specific structure			
		temp = new directory_entry[((sector.BytesPerSector * sector.SectorsPerCluster) / 32)];
			//this allocates for the maximum possible number of items given the number of 
				//clusters in the directory
		//load a cluster, read all entries into the array,
			//stop after the last cluster is processed
			//stop when a free entry is located
			//mark all remaining entries as blank
		keep_going = 1;
	}
	else
	{
		unsigned int sector_number = sector.FirstRootDirSector;
		unsigned int sector_offset = 0;			
		stuff = new dir_item[(sector.BytesPerSector * sector.RootDirSectors) / 32];
		temp = new directory_entry[sector.BytesPerSector / 32];
		keep_going = 1;
	}
	do
	{	//need a loop to check each cluster of the directory
		if (current_directory != 0)
		{	
			load_cluster(fat_disk, drive_num, cluster_check, (unsigned char*)temp);
		}
		else
		{
			fat_disk->read_sector(drive_num, sector_number + sector_offset, (unsigned int*)temp);
		}
		//examine each entry and see what needs to be done with each one
		//check to see if it is a long name or a short name
		//if it is long, add the letters the a running total character array
		//it it is short, determine what type it is (volume id, directory, file, retard)
		for (unsigned int entry = 0;
			entry < ((sector.BytesPerSector) / 32);
			entry++)
		{
			if (temp[entry].short_name[0] == 0x00)
			{
				break;
			}
			if (temp[entry].short_name[0] != 0xE5)
			{
				if ( ((temp[entry].attribute & (ATTR_LONG_NAME_MASK)) == (ATTR_LONG_NAME))  )
				{	//active long name part (this will take quite a bit of work)
					//display("LFN\n");
					//display("Longname attribute: ");
					//PrintNumber(((long_directory*)temp)[entry].attr);
					//display("\n");
					if ((temp[entry].attribute & 0x40) == 0x40)
					{	//this is the last entry (but the last entry is the first entry that this will 			 
						//find because the entries are arranged in a backwards fashion
						long_name = extract_filename(&((long_directory*)temp)[entry]);
							//make the longname string to the temp string
							//because there are no additional letters
						long_check = ((long_directory*)temp)[entry].checksum;
							//set the checksum number
					}
					else if (long_check != 0)
					{//check to make sure the checksum for this element and the first element match
						//display("Second part of a long filename\n");
						unsigned short *temp_longname;
						if ( long_check == ((long_directory*)temp)[entry].checksum )
						{	//only do operations on this if the checksum matches
							unsigned int a, b;
							temp_longname = extract_filename(&((long_directory*)temp)[entry]);
								//place this before the currently existing string
							lengthen = long_name;
							long_name = precatenatew(lengthen, temp_longname);
							delete [] lengthen;
						}
						else
						{	//checksum does not match
							long_check = 0;
						}
						//if the checksum matches, figure out how many letters are valid
						//if it does not match, zero out the checksum
						//copy longname data to a temporary buffer
						//resize the longname buffer
						//copy the current element into the longname buffer
						//copy the temporary buffer back into the longname buffer
					}
				}
				else
				{	//what is it besides an active long name?
					//for everything else besides the invalid case
						//check for a previously read long filename
						//check to see that the long filename checksum matches
						//apply filename and item attributes to the master list
					if ( (temp[entry].attribute & (ATTR_DIRECTORY | ATTR_VOLUME_ID) ) == 0x00)
					{	//file
						if (long_check != 0)
							display("Previous long filename detected\n");
						display("FILENAME: ");
						display((char *)temp[entry].short_name);
						display(",\tSize: ");
						PrintNumber(temp[entry].file_size);
						display("\n");
						stuff[final_entry] = prepare_entry(temp[entry]);
						//PrintNumber(stuff[final_entry].anything1);
						//display("\n");
						stuff[final_entry].type = DIR_ITEM_FILE;
						final_entry++;						
					}
					else if ( (temp[entry].attribute & (ATTR_DIRECTORY | ATTR_VOLUME_ID) ) == ATTR_DIRECTORY)
					{	//directory
						if (long_check != 0)
							display("Previous long filename detected\n");
						display("FOLDER: ");
						display((char *)temp[entry].short_name);
						display(", ");
						PrintNumber(((unsigned long)temp[entry].first_cluster_high * 0x10000) + 
							(unsigned long)temp[entry].first_cluster_lo);
						display("\n");
						stuff[final_entry] = prepare_entry(temp[entry]);
						stuff[final_entry] = prepare_entry(temp[entry]);
						//PrintNumber(stuff[final_entry].anything1);
						//display("\n");
						stuff[final_entry].type = DIR_ITEM_FOLD;
						final_entry++;
					}
					else if ( (temp[entry].attribute & (ATTR_DIRECTORY | ATTR_VOLUME_ID) ) == ATTR_VOLUME_ID)
					{	//volume label
						if (long_check != 0)
							display("Previous long filename detected\n");
						display("VOLUME: ");
						display((char *)temp[entry].short_name);
						display("\n");
					}
					else
					{	//INVALID
						display("INVALID entry\n");
					}
				}
			}
		}
		if (current_directory != 0)
		{
			//load the next cluster information
			cluster_check = get_cluster_entry(fat_disk, drive_num, cluster_check);
			if (is_eof(cluster_check) == 0)
				keep_going = 0;
		}
		else
		{
			sector_offset++;
			if (sector_offset >= (sector.RootDirSectors))
				keep_going = 0;
		}
	} while (keep_going == 1);				
	stuff[final_entry].type = DIR_ITEM_INVA;	//this signals the last entry
	delete [] temp;
	return stuff;
}

unsigned short *fat::extract_filename(long_directory *entry)
{	//extracts the long filename from a long filename entry
	//if this is performed on a non long filename entry, the results would be very weird
	unsigned short *temp_string;
	unsigned long entry_length;
	for (int counter = 0; counter < 5; counter++)
	{							
		if ( entry->name[counter] != 0xFF)
			entry_length++;
	}
	for (int counter = 0; counter < 6; counter++)
	{							
		if ( entry->name2[counter] != 0xFF)
			entry_length++;
	}
	for (int counter = 0; counter < 2; counter++)
	{
		if ( entry->name3[counter] != 0xFF)
			entry_length++;
	}	//figure out how many letters in this entry are valid
	temp_string = new unsigned short[entry_length + 1];
	entry_length = 0;
	for (int counter = 0; counter < 5; counter++)
	{							
		if ( entry->name[counter] != 0xFF)
		{
			temp_string[entry_length] = entry->name[counter];
			entry_length++;
		}
	}
	for (int counter = 0; counter < 6; counter++)
	{							
		if ( entry->name2[counter] != 0xFF)
		{
			temp_string[entry_length] = entry->name2[counter];
			entry_length++;
		}
	}
	for (int counter = 0; counter < 2; counter++)
	{
		if ( entry->name3[counter] != 0xFF)
		{
			temp_string[entry_length] = entry->name3[counter];
			entry_length++;
		}
	}
	temp_string[entry_length] = 0xFFFF;
	return temp_string;
}

dir_item fat::prepare_entry(directory_entry &sample)
{	//if there is a long name that belongs here, then the caller of this function can deallocate the memory for the
		//filename and extension and allocate it itself
	dir_item ret_val;
	char *temp;
	unsigned int a;
	ret_val.size = sample.file_size;
	ret_val.permissions = sample.attribute;
	ret_val.type = DIR_ITEM_FILE;
	ret_val.date_crt1 = ((sample.date_creation & 0xFFE0)>>5) + (1980<<4);
	ret_val.date_crt2 = ((sample.date_creation & 0xF)<<17) + 
		((sample.time_creation & 0xF800)>>11) +
		((sample.time_creation & 0x7E0)<<1) +
		((sample.time_creation & 0x1F)<<1); 
	ret_val.date_mod1 = ((sample.write_date & 0xFFE0)>>5) + (1980<<4);
	ret_val.date_mod2 = ((sample.write_date & 0xF)<<17) + 
		((sample.write_time & 0xF800)>>11) +
		((sample.write_time & 0x7E0)<<1) +
		((sample.write_time & 0x1F)<<1); 
	ret_val.date_acc1 = ((sample.access_date & 0xFFE0)>>5) + (1980<<4);
	ret_val.date_acc2 = ((sample.access_date & 0xF)<<17);
	ret_val.anything1 = (((unsigned long)sample.first_cluster_high * 0x10000) + 
							(unsigned long)sample.first_cluster_lo);
	temp = new char[9];			//maximum possible size after the null character is added
	for (a = 0; a < 8; a++)
	{
		if (sample.short_name[a] == ' ')
			break;
		temp[a] = sample.short_name[a];
	}
	temp[a] = '\0';
	ret_val.name = new char[a + 1];
	strcpy(ret_val.name, temp);
	for (a = 0; a < 3; a++)
	{
		temp[a] = sample.short_name[a + 8];
	}
	temp[a] = '\0';
	ret_val.extension = new char[a + 1];
	strcpy(ret_val.extension, temp);
	return ret_val;
}

int fat::get_cluster_entry(disk *local_drive, unsigned int drive_num, unsigned int cluster_num)
{	//this function returns the value of the cluster entry
	unsigned int fat_offset;
	unsigned int fat_sec_num, fat_sec_offset;
	unsigned char *buffer;
	unsigned short cluster_value;
	unsigned int return_value;
	//fat_sec_num is the sector number that contains the entry for cluster cluster_num (first FAT)
		//for the second or third FAT add FatSize to this number the appropriate number of times
	//fat_sec_offset is the offset from the sector previously mentioned
		//it is relative to sector 0 of the FAT volume
	if (isFat12() != 0)
	{	//fat16 or fat32
		if (isFat16() == 0)
		{
			fat_offset = cluster_num * 2;
		}
		else if (isFat32() == 0)
		{
			fat_offset = cluster_num * 4;
		}
		fat_sec_num = sector.ReservedSectorCount + (fat_offset / sector.BytesPerSector);// + 1;
			//TODO: check this to see if the + 1 is necessary
		fat_sec_offset = fat_offset % sector.BytesPerSector;
		//read the sector required for getting the cluster entry
		buffer = new unsigned char[0x1000];
		local_drive->read_sector(drive_num, fat_sec_num, (unsigned int*)buffer);
		if (isFat16() == 0)
		{	//hopefully this will truncate properly (16 bits only required)
			return_value = (unsigned int)*((unsigned short*) &buffer[fat_sec_offset]);
			delete[] buffer;
			return return_value;
		}
		else
		{
			return_value = (*((unsigned int*) &buffer[fat_sec_offset]) & 0x0FFFFFFF);
			delete[] buffer;
			return return_value;
		}
	}
	else
	{	//fat12
		fat_offset = cluster_num + (cluster_num / 2);	//non floating point multiply by 1.5 rounds down
		fat_sec_num = sector.ReservedSectorCount + (fat_offset / sector.BytesPerSector);
		fat_sec_offset = fat_offset % sector.BytesPerSector;
//		display("\nFAT entry @ sector: ");
//		PrintNumber(fat_sec_num);
//		display(", offset: ");
//		PrintNumber(fat_sec_offset + 0x400);
		buffer = new unsigned char[0x1000];
		local_drive->read_sector(drive_num, fat_sec_num, (unsigned int*)buffer);
		if (fat_sec_offset == (sector.BytesPerSector - 1))
		{	//cluster access spans a sector boundary
			display("FAT12: Cluster entry spans a sector boundary, loading second sector\n");
			local_drive->read_sector(drive_num, fat_sec_num + 1, 
				(unsigned int*)&buffer[sector.BytesPerSector / sizeof(unsigned short)]);
		}
		//gcc likes to make sure that short * are aligned by their size
			//which caused problems with FAT12 using unaligned pointers
		cluster_value = //((unsigned char) buffer[fat_sec_offset / sizeof(unsigned char)]);// +
//		cluster_value += ((unsigned char) buffer[fat_sec_offset / sizeof(unsigned char) + 1])<<8;
						*((unsigned short *) &buffer[fat_sec_offset]);
		delete[] buffer;
//		display("\nCluster entry unmasked: ");
//		PrintNumber(cluster_value);
		if (cluster_num & 0x0001)
		{
			//display("\tOdd cluster number\n");
			cluster_value = (cluster_value >> 4);// + (cluster_value & 0xFF)<<4;
		}
		else
		{
			//display("\tEven cluster number\n");
			cluster_value = cluster_value & 0x0FFF;
		}
//		display("\nCluster entry value: ");
//		PrintNumber(cluster_value);
//		display("\n");
		return cluster_value;
	}
	return INVALID_DRIVE;
}

int fat::is_eof(unsigned int cluster_value)
{	//0 - eof
	//-1 - not eof
//	display("Testing for eof/eoc: ");
//	PrintNumber(cluster_value);
//	display("\n");
	if (isFat12() == 0)
	{
		if (cluster_value >= 0x0FF8)
			return 0;
	}
	else if (isFat16() == 0)
	{
		if (cluster_value >= 0xFFF8)
			return 0;
	}
	else if (isFat32() == 0)
	{
		if (cluster_value >=0x0FFFFFF8)
			return 0;
	}
	return -1;
}

int fat::is_bad_cluster(unsigned int cluster_value)
{
	if (isFat12() == 0)
	{
		if (cluster_value == 0x0FF7)
			return 0;
	}
	else if (isFat16() == 0)
	{
		if (cluster_value == 0xFFF7)
			return 0;
	}
	else if (isFat32() == 0)
	{
		if (cluster_value == 0x0FFFFFF7)
			return 0;
	}
	return -1;
}

unsigned int fat::long_name_checksum(unsigned char *pFcbName)
{	//takes an 11 byte character array and calculates the checksum for it
	short FcbNameLen;
	unsigned char Sum;
	Sum = 0;
	for (FcbNameLen=11; FcbNameLen!=0; FcbNameLen--) {
	        // NOTE: The operation is an unsigned char rotate right
	        Sum = ((Sum & 1) ? 0x80 : 0) + (Sum >> 1) + *pFcbName++;
	}
	return (Sum);
}

int fat::is_mounted()
{	//-1 is not mounted, 1 is mounted
	return valid;
}

int fat::is_short_name_valid(char *file_name)	//checks an 11 byte name for valid characters
{
	for (int counter = 0; counter < 11; counter++)
	{
		if ((counter == 0) && (file_name[counter] == 0x05))
		{
			//this is ok
		}
		else
		{
			if (file_name[counter] < 0x20)
			{
				return -1;
			}
			switch(file_name[counter])
			{
				case 0x22: case 0x2A: case 0x2B: case 0x2C: case 0x2E: case 0x2F: case 0x3A: case 0x3B:
				case 0x3C: case 0x3D: case 0x3E: case 0x3F: case 0x5B: case 0x5C: case 0x5D: case 0x7C:
				{
					return -1;
					break;
				}
				default:
					break;
			}
		}
	}
	return 0;	//indicate that the filename checked out to be good
}

int fat::load_cluster(disk *somewhere, unsigned int drive_number, unsigned long cluster_number, unsigned char *buffer)
{	//buffer is required to be already allocated based on sectors per cluster * bytes per sector + bytes_per_sector(from disk*)
	//the extra bytes per sector is to provide padding in the event of a sector size mismatch between filesystem and disk
	unsigned int offset = 0;	//this is the offset to use for the buffer
	unsigned int sector_number = FirstSectorOfCluster(cluster_number, sector.SectorsPerCluster, sector.FirstDataSector);
//	display("\nLoading cluster ");
//	PrintNumber(cluster_number);
//	display("\n");
	for (int counter = 0; counter < sector.SectorsPerCluster; counter++)
	{	//for now assume only one sector can be read at a time
//		display("\tLoad sector ");
//		PrintNumber(sector_number);
//		display("\tfirst byte:");
//		PrintNumber(sector_number * sector.BytesPerSector);
//		display("\n");
		if (somewhere->read_sector(drive_number, sector_number, (unsigned int*)&buffer[offset]) == -1)
		{
			display("ERROR: load_cluster(): read_sector\n");
			return -1;
		}
		sector_number++;
		if (somewhere->bytes_per_sector(drive_number) == -1)
		{
			display("bytes_per_sector returned -1\n");
			offset += sector.BytesPerSector;
		}
		else
		{
			offset += somewhere->bytes_per_sector(drive_number);
		}
	}
	return 0;
}

unsigned long fat::find_dir(const char* directory)
{
	unsigned int eof;
	dir_item *directory_search;
	display("Attempting to locate ");
	display((char *)directory);
	display("\n");
	if (isFat12() == 0)
	{
		eof = 0x0FF8;
	}
	else if (isFat16() == 0)
	{
		eof = 0xFFF8;
	}
	else if (isFat32() == 0)
	{
		eof = 0x0FFFFFF8;
	}
	else
	{
		eof = 0xFFFFFFFF;
	}
	directory_search = list_contents();
	for (unsigned int a = 0; directory_search[a].type != DIR_ITEM_INVA; a++)
	{
		if (directory_search[a].type == DIR_ITEM_FOLD)
		{
			display("Compare \'");
			display(directory_search[a].name);
			display("\' to \'");
			display(directory);
			display("\'...\t");
			if (stringCompare(directory_search[a].name, directory) == 0)
			{
				display("Match ");
				PrintNumber(directory_search[a].anything1);
				display("\n");
				//figure out a way to get the filesystem specific data from the non-specific data
				//might require the emplacement of some general purpose values in the non-specific data
				return (directory_search[a].anything1);	//return the first cluster of the folder
			}
			display("No match\n");
		}
	}
	delete [] directory_search;
	return eof;	//unable to find the directory
}

dir_item *fat::find_file(const char* filename)
{
	unsigned int eof;
	dir_item *filename_search;
	dir_item *ret_me;
	ret_me = new dir_item;
	display("Attempting to locate ");
	display((char *)filename);
	display("\n");
	if (isFat12() == 0)
	{
		eof = 0x0FF8;
	}
	else if (isFat16() == 0)
	{
		eof = 0xFFF8;
	}
	else if (isFat32() == 0)
	{
		eof = 0x0FFFFFF8;
	}
	else
	{
		eof = 0xFFFFFFFF;
	}
	filename_search = list_contents();
	for (unsigned int a = 0; filename_search[a].type != DIR_ITEM_INVA; a++)
	{
		if (filename_search[a].type == DIR_ITEM_FILE)
		{
			display("Compare \'");
			display(filename_search[a].name);
			display("\' to \'");
			display(filename);
			display("\'...\t");
			if (stringCompare(filename_search[a].name, filename) == 0)
			{
				display("Match ");
				PrintNumber(filename_search[a].anything1);
				display("\n");
				//figure out a way to get the filesystem specific data from the non-specific data
				//might require the emplacement of some general purpose values in the non-specific data
				memcopy((void*)ret_me, (void*)&filename_search[a], sizeof(dir_item));
				delete [] filename_search;
				return (ret_me);//.anything1);	//return the first cluster of the file
			}
			display("No match\n");
		}
	}
	delete [] filename_search;
	return 0;	//unable to find the filename
}


int fat::enter_directory(const char* dir)	//enters the directory given
{			//. and .. will be processed accordingly unless it is the root directory
			//if it is the root directory, those directories will not be found
	unsigned int dir_cluster;	
	dir_cluster = find_dir(dir);
	if (is_eof(dir_cluster) == 0)
		return -1;	//indicate that the directory was not found for some reason
					//TODO: make some error values for reasons that would not work
					//could it also indicate an empty folder?
	//TODO: append the directory name and whatever to the current directory
	current_directory = dir_cluster;
	depth++;
	return 0;
}

krnl_FILE *fat::open_file(char *filename)
{
	krnl_FILE *descriptor;
	descriptor = new krnl_FILE;
	descriptor->filename = new char[strlen(filename) + 1];
	strcpy(descriptor->filename, filename);
	descriptor->offset = 0;
	descriptor->fs_spec = (void*)find_file(filename);
	if (is_eof(((dir_item*)(descriptor->fs_spec))->anything1) == 0)
	{
		display("First cluster of file is EOF\n");
		return 0;
	}
	descriptor->buffer_length = get_buffer_size();
	if (descriptor->buffer_length == 0)
	{
		return 0;
	}
	descriptor->buffer_offset = 0;
	descriptor->buffer = get_buffer(0, descriptor);
	descriptor->length = ((dir_item*)(descriptor->fs_spec))->size;
	if (descriptor->buffer != 0)
		return descriptor;
	display("Unable to allocate cluster buffer or other unexpected error\n");
	return 0;
}

unsigned int fat::eof(krnl_FILE *descriptor)
{
	if (descriptor->offset >= descriptor->length)
	{
		return 0;
	}
	return 1;
}

unsigned char fat::get_b(krnl_FILE *descriptor, filesystem *owner)
{
	unsigned char value;
	if (descriptor->offset >= descriptor->length)
	{
		return -1;
	}
	if (0)//((descriptor->offset + 1) > descriptor->buffer_length)
	{	//something in this if block affects the way the next cluster is loaded or not loaded
		display("TABLE!@!@ offset:");
		PrintNumber(descriptor->offset);
		display("\tbuffer: ");
		PrintNumber(descriptor->buffer_offset);
		display("\tbuffer_point:");
		PrintNumber((descriptor->offset - descriptor->buffer_offset));
		display("\nlength: ");
		PrintNumber(descriptor->buffer_length);
		display("\tbuffer address: ");
		PrintNumber((unsigned long)descriptor->buffer);
		display("\n");
		Delay(2000);
	}
	if ((descriptor->offset - descriptor->buffer_offset + sizeof(value) - 1) < descriptor->buffer_length)
	{
		value = descriptor->buffer[descriptor->offset - descriptor->buffer_offset];
		descriptor->offset += sizeof(value);
		return value;
	}
	else if ((descriptor->offset - descriptor->buffer_offset + sizeof(value) - 1) == descriptor->buffer_length)
	{	//value lies completely in the next cluster
		delete [] descriptor->buffer;		
		descriptor->buffer = get_buffer(descriptor->offset, descriptor);
		descriptor->buffer_offset = (unsigned int)(descriptor->offset / descriptor->buffer_length);
		descriptor->buffer_offset *= descriptor->buffer_length;
		value = descriptor->buffer[descriptor->offset - descriptor->buffer_offset];
		descriptor->offset += sizeof(value);
		return value;
	}
	else
	{
		display("BOUNDARY CASE\noffset:");
		PrintNumber(descriptor->offset);
		display("\tbuffer: ");
		PrintNumber(descriptor->buffer_offset);
		display("\nlength: ");
		PrintNumber(descriptor->buffer_length);
		display("\tread: ");
		PrintNumber(sizeof(value));
		display("\n");
		for (;;);
	}
}

unsigned short fat::get_w(krnl_FILE *descriptor, filesystem *owner)
{
	unsigned short ret_val = 0;
	unsigned short temp = 0;
	ret_val = get_b(descriptor, owner);
	temp = (unsigned short)get_b(descriptor, owner);
	ret_val += temp<<8;
	return ret_val;
}

unsigned long fat::get_dw(krnl_FILE *descriptor, filesystem *owner)
{
	unsigned long ret_val = 0;
	unsigned long temp = 0;
	ret_val = get_b(descriptor, owner);
	for (int a = 0; a < 3; a++)
	{
		temp = get_b(descriptor, owner);
		ret_val += temp<<(8*(a+1));
	}
	return ret_val;
}

unsigned long fat::seek(krnl_FILE *descriptor, unsigned long position, filesystem *owner)
{
	if (descriptor->offset < descriptor->length)
	{
		descriptor->offset = position;
		descriptor->buffer_offset = (unsigned int)(descriptor->offset / descriptor->buffer_length);
		descriptor->buffer_offset *= descriptor->buffer_length;
		delete [] descriptor->buffer;
		descriptor->buffer = get_buffer(descriptor->offset, descriptor);
		return position;
	}
	else
	{
		return -1;
	}
}

unsigned int fat::get_buffer_size()
{	//returns the size of the block that file data is passed in (in bytes)
	//return the size of a cluster
	if (valid == 1)
	{
		return (sector.BytesPerSector * sector.SectorsPerCluster);
	}
	return 0;	//0 for size indicates that no data will be transferred because the filesystem is not usable right now
}


unsigned char *fat::get_buffer(unsigned int offset, krnl_FILE *file)
{	//returns the buffer that contains the proper offset into the file
	unsigned char *buffer;
	buffer = new unsigned char[get_buffer_size() / sizeof(unsigned char)];
//	display("Cluster buffer address: ");
//	PrintNumber((unsigned long)buffer);
//	display("\n");
	unsigned int cluster_number = offset / get_buffer_size() + 1;
		//which cluster number of the file needs to be loaded?
	unsigned int current_cluster = ((dir_item*)(file->fs_spec))->anything1;
		//the current cluster (initialize to the first cluster)
	unsigned int cluster_count = 1;		//the cluster number order (first, second, third, etc)
//	display("Looking for cluster order number ");
//	PrintNumber(cluster_number);
//	display("\n");
	//cluster (cluster entry)
	//1 (5)
	//5 (4)
	//4 (eof)
	//eof (###)
	while ( is_eof(current_cluster) != 0 )
	{
		//check the current cluster number (first second thrid whatever)
			//when it matches the desired cluster number (previously calculated and stored in cluster_number)
		//load that cluster into the buffer and return
		if (cluster_count == cluster_number)
		{	//desired cluster number has been found, load it into the buffer and return
			load_cluster(fat_disk, drive_num, current_cluster, (unsigned char*)buffer);
//			display("\tafter: ");
//			PrintNumber(buffer[0]);
//			display("\n"); 
			return buffer;
		}
//		display("Cluster: ");
//		PrintNumber(current_cluster);
//		display(", ");
//		PrintNumber(get_cluster_entry(fat_disk, drive_num, current_cluster));
//		display("\n");
		current_cluster  = get_cluster_entry(fat_disk, drive_num, current_cluster);
		cluster_count++;
	}
	delete [] buffer;
	return 0;	//cluster number not found
}

//this is a read only driver, it is not concerned with whether the disk was cleanly dismounted or not
	//or if access errors occurred

	
//a read only driver will not perform drive formatting

//fat32 backup boot sector is located at sector 6 of the volume

//root directory and directory entries

