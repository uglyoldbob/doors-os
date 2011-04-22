#include "floppy.h"
#include "memory.h"
#include "dma.h"
extern unsigned int inportb(unsigned int port);		//entrance.asm
extern unsigned int outportb(unsigned int value, unsigned int port);	//entrance.asm
extern void WaitFloppyInt();	//entrance.asm

extern unsigned int timer;	//entrance.asm

	//waits for the floppy drive to signal with an interrupt
//actually only the bottom two bytes of port and the bottom byte of the return value is used

//offsets for the various registers
#define STATUS_REG_A            0x0000 /*PS2 SYSTEMS*/
#define STATUS_REG_B            0x0001 /*PS2 SYSTEMS*/
#define DIGITAL_OUTPUT_REG      0x0002
#define TAPE_DRIVE_REGISTER     0x0003
#define MAIN_STATUS_REG         0x0004
#define DATA_RATE_SELECT_REG    0x0004 /*PS2 SYSTEMS*/
#define DATA_REGISTER           0x0005
#define DIGITAL_INPUT_REG       0x0007 /*AT SYSTEMS*/
#define CONFIG_CONTROL_REG      0x0007 /*AT SYSTEMS*/
#define PRIMARY_RESULT_STATUS   0x0000
#define SECONDARY_RESULT_STATUS 0x0000

//command bytes for the floppy disk controller
#define FIX_DRIVE_DATA          0x03
#define CHECK_DRIVE_STATUS      0x04
#define CALIBRATE_DRIVE         0x07
#define CHECK_INTERRUPT_STATUS  0x08
#define FORMAT_TRACK            0x4D
#define READ_SECTOR             0x66
#define READ_DELETE_SECTOR      0xCC
#define READ_SECTOR_ID          0x4A
#define READ_TRACK              0x42
#define SEEK_TRACK              0x0F
#define WRITE_SECTOR            0xC5
#define WRITE_DELETE_SECTOR     0xC9

#define DISK_PARAMETER_ADDRESS 0x000FEFC7 /* location where disk parameters */
                                          /* is stored by bios */

typedef struct
{	//the table as BIOS fills it out at the above address
  unsigned char steprate_headunload;
  unsigned char headload_ndma;
  unsigned char motor_delay_off; /*specified in clock ticks*/
  unsigned char bytes_per_sector;
  unsigned char sectors_per_track;
  unsigned char gap_length;
  unsigned char data_length; /*used only when bytes per sector == 0*/
  unsigned char format_gap_length;
  unsigned char filler;
  unsigned char head_settle_time; /*specified in milliseconds*/
  unsigned char motor_start_time; /*specified in 1/8 seconds*/
}__attribute__ ((packed)) floppy_parameters;

floppy_parameters floppy_disk; 
/*declare variable of floppy_parameters type*/
//will be used for all future floppy disk access

waitRecieveFloppy(unsigned int base)
{
	//while ((inportb(base + CHECK_DRIVE_STATUS) & 0xC0) != 0xC0){};
	unsigned int temp;
	while (1)
	{
		temp = inportb(base + CHECK_DRIVE_STATUS);
		if ((temp & 0xD0) == 0xD0)	//only let it retrieve data while the controller is not busy?
		{
			return;
		}
		else if ((temp & 0xD0) == 0x80)
		{
			display("Controller is waiting for data, this is unexpected, MSR:");
			PrintNumber(temp);
			display("\n");
		}
		Delay(1000);
	}
}


sendFloppyCommand(unsigned int base, unsigned char command)
{
	unsigned int temp;
	while (1)
	{
		temp = inportb(base + CHECK_DRIVE_STATUS);
		if ((temp & 0xC0) == 0xC0)
		{
			display("Controller wants to give, this is unexpected, MSR:");
			PrintNumber(temp);
			display("\n");
		}
		else if ((temp & 0xC0) == 0x80)
		{
			outportb(command, base + DATA_REGISTER);
			break;
		}
		Delay(1000);
	}
}

check_floppy_status(unsigned int base, unsigned int *st0, unsigned int *cylinder)
{	//performs the FDC instruction and returns all applicable results
	//waitSendFloppy(base);
	sendFloppyCommand(base, CHECK_INTERRUPT_STATUS);
	waitRecieveFloppy(base);
	*st0 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*cylinder = inportb(base+DATA_REGISTER);
}

void configure_drive(unsigned int base)
{
  sendFloppyCommand(base,FIX_DRIVE_DATA);/*config/specify command*/
  sendFloppyCommand(base,floppy_disk.steprate_headunload);
  sendFloppyCommand(base,floppy_disk.headload_ndma);	//set bit 0 for nondma transfer, clear it for DMA transfer
	//no results
  return;
}

void calibrate_drive(unsigned int base,char drive)
{	//drive = 0,1,2,3
	unsigned int st0, cylinder, command;
	command = (0x10<<drive);
	command += 0x0C + drive;
	outportb(command, base + DIGITAL_OUTPUT_REG);
	//make sure motor is turned on
	do
	{
		sendFloppyCommand(base,CALIBRATE_DRIVE); /*Calibrate drive*/
  	sendFloppyCommand(base,drive);
		WaitFloppyInt();
  	check_floppy_status(base,&st0,&cylinder); /*check interrupt status and
                                                store results in global variables
                                                st0 and cylinder*/
		if (st0 != 0x20)
		{
			display("Error in calibrate drive, st0:");
			PrintNumber(st0);
		}
	} while (cylinder != 0);
	//repeat until the floppy drive is over cylinder 0
	return;
}

void seek_to_cylinder(unsigned int cylinder, unsigned int head, unsigned int base, unsigned char drive)
{
	unsigned int st0, cylinder_check;
	do
	{
		sendFloppyCommand(base, SEEK_TRACK);
		sendFloppyCommand(base, head<<2 | drive);
		sendFloppyCommand(base, cylinder);
		WaitFloppyInt();	//wait for the impending interrupt
		check_floppy_status(base, &st0, &cylinder_check);
	} while (cylinder_check != cylinder);
	return;
}

reset_floppy(unsigned int base, char drive)
{
	unsigned int st0, cylinder;	//this will be used for any necessary storage of FDC states
	//reset the floppy disk to a known state
	outportb(0, base + DIGITAL_OUTPUT_REG);
	while (inportb(base + CHECK_DRIVE_STATUS) != 0);
	//should delay int enough for the reset to finish course
	outportb(0x0C, base + DIGITAL_OUTPUT_REG);
	WaitFloppyInt();
	check_floppy_status(base, &st0, &cylinder);
	check_floppy_status(base, &st0, &cylinder);
	check_floppy_status(base, &st0, &cylinder);
	check_floppy_status(base, &st0, &cylinder);
	outportb(0, base + CONFIG_CONTROL_REG);
	configure_drive(base);
	calibrate_drive(base, drive);
}

void getResults(unsigned int *st0, unsigned int *st1, unsigned int *st2, unsigned int *cylinder_r, 
					unsigned int *head_r, unsigned int *sector_r, unsigned int *size_r, unsigned int base)
{
	waitRecieveFloppy(base);
	*st0 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*st1 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*st2 = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*cylinder_r = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*head_r = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*sector_r = inportb(base+DATA_REGISTER);
	waitRecieveFloppy(base);
	*size_r = inportb(base+DATA_REGISTER);
}

void read_track(unsigned int sector_number, unsigned char drive,unsigned int buffer, unsigned int base)
{
	unsigned int st0, st1, st2, cylinder_r, head_r, sector_r, size_r;//, cylinder_check;
	//these store the results from the read command
	unsigned int length = 1;
	unsigned int counter = 0;
	unsigned int command;
	unsigned int cylinder, head, sector;
	sector = (sector_number % floppy_disk.sectors_per_track) + 1;
	cylinder = (sector_number / floppy_disk.sectors_per_track) / 2;	//there are 2 heads on a floppy drive
	head = (sector_number / floppy_disk.sectors_per_track) % 2;			//2 heads on a floppy drive
	for (counter = 0; counter < floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	length *= floppy_disk.sectors_per_track;	//bytes per track
		//size of the sector in bytes
		//will need to be recalculated
	//enable the motor first
	command = (0x10<<drive);
	command += 0x0C + drive;
	outportb(command, DIGITAL_OUTPUT_REG);
	//configure the DMA (channel 2)
	startDMA(buffer, length, 2, 0x45);
	//give the seek track command
	seek_to_cylinder(cylinder, head, base, drive);
	Delay(floppy_disk.head_settle_time);
	//the length of time in milliseconds it takes for the head to settle after moving
	sendFloppyCommand(base, READ_TRACK);
	sendFloppyCommand(base, head<<2|drive);
	sendFloppyCommand(base, cylinder);
	sendFloppyCommand(base, head);
	sendFloppyCommand(base, sector);
	sendFloppyCommand(base, floppy_disk.bytes_per_sector);  /*sector size = 128*2^size*/
	sendFloppyCommand(base, floppy_disk.sectors_per_track); /*last sector*/
	sendFloppyCommand(base, floppy_disk.gap_length);        /*27 default gap3 value*/
	sendFloppyCommand(base, floppy_disk.data_length);       /*default value for data length*/
	WaitFloppyInt();	//wait for the completion of the command
	//check_floppy_status(base, &st0, &cylinder);
	getResults(&st0, &st1, &st2, &cylinder_r, &head_r, &sector_r, &size_r, base);
}

void read_sector(unsigned int sector_number, unsigned char drive,unsigned int buffer, unsigned int base)
{	//starts at sector 0 for sector_number
	unsigned int st0, st1, st2, cylinder_r, head_r, sector_r, size_r;//, cylinder_check;
	//these store the results from the read command
	unsigned int length = 1;
	unsigned int counter = 0;
	unsigned int command;
	unsigned int cylinder, head, sector;
	sector = (sector_number % floppy_disk.sectors_per_track) + 1;
	cylinder = (sector_number / floppy_disk.sectors_per_track) / 2;	//there are 2 heads on a floppy drive
	head = (sector_number / floppy_disk.sectors_per_track) % 2;			//2 heads on a floppy drive
	for (counter = 0; counter < floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
		//size of the sector in bytes
	//enable the motor first
	command = (0x10<<drive);
	command += 0x0C + drive;
	outportb(command, DIGITAL_OUTPUT_REG);
	//configure the DMA (channel 2)
	startDMA(buffer, length, 2, 0x45);
	//give the seek track command
	seek_to_cylinder(cylinder, head, base, drive);
	Delay(floppy_disk.head_settle_time);
	//the length of time in milliseconds it takes for the head to settle after moving
	sendFloppyCommand(base, READ_SECTOR);
	sendFloppyCommand(base, head<<2|drive);
	sendFloppyCommand(base, cylinder);
	sendFloppyCommand(base, head);
	sendFloppyCommand(base, sector);
	sendFloppyCommand(base, floppy_disk.bytes_per_sector);  /*sector size = 128*2^size*/
	sendFloppyCommand(base, floppy_disk.sectors_per_track); /*last sector*/
	sendFloppyCommand(base, floppy_disk.gap_length);        /*27 default gap3 value*/
	sendFloppyCommand(base, floppy_disk.data_length);       /*default value for data length*/
	WaitFloppyInt();	//wait for the completion of the command
	//check_floppy_status(base, &st0, &cylinder);
	getResults(&st0, &st1, &st2, &cylinder_r, &head_r, &sector_r, &size_r, base);
}

void initialize_floppy(unsigned int base)
{
	memcpy(&floppy_disk, (unsigned char *)DISK_PARAMETER_ADDRESS, sizeof(floppy_parameters));
	reset_floppy(base, 0);
	outportb(0, base + DIGITAL_OUTPUT_REG);
}

//add functions to read files from the root directory of the disk
//then multitasking
//virtual memory
//complete keyboard driver
//...
//write functions to write to disk (carefully)
