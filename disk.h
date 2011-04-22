
#ifndef _DISK_H_
#define _DISK_H_

#define INVALID_DRIVE 0xFFFFFFFF
#define NOT_SUPPORTED 0xFFFFFFFE
	//the not supported error code must be different from -1 (general failure)

//it is perfectly acceptable and suggested that drivers that need extra input create a file to store those settings.
	//this would be a great place for the customization of drivers, a gui config program can optionally be added
		//please keep in mind however, that there is no GUI for doors yet.

struct sectorReturn
{	//this is used to send and recieve sectors
	unsigned char *data;
	unsigned long  size;	//the size of the buffer
};

struct driveData
{	//stores data required to access a drive
	char 					*name;					//not case sensitive (I think case sensitivity here is a bad idea)
	unsigned char  drive_num;	//always present and useful
	
	//function pointer to the function selector
};

struct sectorReturn readSector(unsigned char driveNum, unsigned long sectorNumber);

class disk
{	//this class will use a lot of virtual functions
	//derived classes will be treated as base classes, eliminating the details that 
			//the upper level code routines don't care about
	//any features not supported by a device driver should return an error code of NOT_SUPPORTED
		//or 0 in the case of identify driver
		//or something to indicate error
	public:
		disk();
		virtual ~disk();	//virtual destructor
		virtual int read_sector(unsigned int drive_num, unsigned long sector_number, unsigned int *buffer);
		virtual char *identify_driver();	//returns a string identifying the driver
		virtual int number_drives();	//returns the number of drives addressed by the driver
		virtual int get_drive_number(int drive_order);	//returns the external disk number (0-0xFFFFFFFE) given the
																										// internal disk number (0-num_disks)
		virtual int bytes_per_sector(unsigned int drive_num);	//returns the size of a sector in bytes
	private:
	protected:
		char 					*drive_name;
		unsigned int get_drive_num();	//returns a usable drive number
};

#endif
