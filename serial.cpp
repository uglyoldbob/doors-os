//serial.cpp
#include "serial.h"
#include "entrance.h"
#include "message.h"

message add_me_serial;

serial kellogs;	//so nice isnt it?

serial::serial()
{	//initialize the serial port (COM1)
	add_me_serial.who = SERIAL;
	add_me_serial.fields = 1;
	//this should be extended later on to provide support for many serial ports
	/*outportb(0x00, COM1_PORT + 1);    // Disable all interrupts
	outportb(0x80, COM1_PORT + 3);    // Enable DLAB (set baud rate divisor)
	outportb(0x0C, COM1_PORT + 0);    // Set divisor to 12 (lo byte) 9600 baud
	outportb(0x00, COM1_PORT + 1);    //                  (hi byte)
	outportb(0x03, COM1_PORT + 3);    // 8 bits, no parity, one stop bit
	outportb(0xC7, COM1_PORT + 2);    // Enable FIFO, clear them, with 14-byte threshold
	outportb(0x0B, COM1_PORT + 4);    // IRQs enabled, RTS/DSR set
*/
	initialize(COM1_PORT, LCR_DEFAULT, FCR_DEFAULT, 0x0C, MCR_DEFAULT, IER_DEFAULT);
}

void serial::initialize(unsigned int base_port, int line_control, int fifo, unsigned int speed, int modem_control, int enable_ints)
{	//TODO: read initial states for the com port and figure out why input is being disabled

	unsigned int stat1, stat2, stat3, stat4, stat5, stat6, stat7;
	stat1 = inportb(base_port + LINE_CONTROL);
	if ((stat1 & 0x80) == 0x80)
	{
		outportb(stat1 & 0x7F, base_port + LINE_CONTROL);
	}
	stat2 = inportb(base_port + INT_ENABLE);
	stat3 = inportb(base_port + INT_ID);
	stat4 = inportb(base_port + LINE_CONTROL);
	stat5 = inportb(base_port + MODEM_CONTROL);
	stat6 = inportb(base_port + LINE_STATUS);
	stat7 = inportb(base_port + MODEM_STATUS);
	//3, 0, c1, 3, b, 0, b0
	display("\nSerial port status:\n");
	PrintNumber(stat1);
	display(", ");
	PrintNumber(stat2);
	display(", ");
	PrintNumber(stat3);
	display(", ");
	PrintNumber(stat4);
	display(", ");
	PrintNumber(stat5);
	display(", ");
	PrintNumber(stat6);
	display(", ");
	PrintNumber(stat7);
	display("\n");

	outportb(0x00, base_port + INT_ENABLE);	//disable all com port interrupts
	outportb(line_control | LCR_DIVISOR_LATCH, base_port + LINE_CONTROL);	
		//setup data length, parity, stop bits, 
		//also set the divisor latch so the data speed can be set
	outportb(fifo | FCR_CLEAR_RECEIVE | FCR_CLEAR_SEND, base_port + FIFO_CONTROL);
	outportb(speed & 0xFF, base_port + DIVISOR_LATCH_LOW_BYTE);
	outportb(speed>>8, base_port + DIVISOR_LATCH_HI_BYTE);
	outportb(line_control, base_port + LINE_CONTROL);
	outportb(modem_control | MCR_AUX2 | MCR_RTS | MCR_DTR, base_port + MODEM_CONTROL);
	outportb(enable_ints, base_port + INT_ENABLE);	//enable interrupts requested
	
}

EXTERNC void ser_handler();

asm(".text");
asm(".globl ser_handler");
asm(".align 4");
asm("ser_handler:");
asm("	pusha");
asm("	call handle_serial");
asm("	popa");
asm("	iret");
/*
int c;
 do { c = inportb(PORT1 + 5);
      if (c & 1) {buffer[bufferin] = inportb(PORT1);
		  bufferin++;
		  if (bufferin == 1024) {bufferin = 0;}}
    }while (c & 1);
 outportb(0x20,0x20);
*/
EXTERNC void handle_serial()
{
	//display("?");
	unsigned int result = 0;
	result = inportb(COM1_PORT + INT_ID);
	unsigned int status;	//stores the status of whatever needs checking
	if ((result & IID_NO_INT) != IID_NO_INT)
	{	//interrupt pending
		//display("\nInterrupt pending: ");
		//PrintNumber(result);
		//display("\n");
		if ((result & 0x06) == IID_MODEM)
		{
			
		}
		if ((result & 0x06) == IID_REC_STAT)
		{
			//status = inportb(COM1_PORT);
			
		}
		if ((result & 0x06) == IID_TRANSMIT)
		{
			
		}
		if ((result & 0x06) == IID_RECEIVED)
		{
			add_me_serial.data1 = kellogs.read_serial();
			add_system_event(&add_me_serial);
		}
	}
	outportb(0x20,0x20);	//signal end of interrupt
	//display("+");
}

int serial::serial_received()
{
   return (inportb(COM1_PORT + 5) & 1);
}

char serial::read_serial()
{
   while (serial_received() == 0);

   return inportb(COM1_PORT);
}



int serial::is_transmit_empty()
{
   return (inportb(COM1_PORT + 5) & 0x20);
}

void serial::write_serial(char a)
{
   while (is_transmit_empty() == 0);

   outportb(a, COM1_PORT);
}

//divisor latch access bit
	//set with the line control register
//115,200 maximum bits per second

/*
interrupt enable register
INT_ENABLE
bit 7 reserved
bit 6 reserved
bit 5 enable low power mode (16750) - reserved
bit 4 enable sleep mode (16750) - reserved
bit 3 modem status interrupt
bit 2 enable receiver line status interrupt
bit 1 enable transmitter holding register empty interrupt
bit 0 enable received data available interrupt
*/

/*
interrupt identification register
INT_ID
bits 6 7 (
		00 = no fifo, 
		01 = fifo there but unusable, 
		11 = fifo enabled)
bit 5 64 byte fifo enables (16750 only)
bit 4 reserved
bit 3 reserved except 16750 - time-out interrupt pending
bit 1 2 (
		00 modem status, 
		01 transmitter holding register empty,
		10 received data, 
		11 receiver line status
bit 0 interrupt not pending (0 = interrupt pending)
*/

/*
first in first out control register (FCR)
FIFO_CONTROL
bits 7 6 interrupt trigger level
		0 0 = 1 byte
		0 1 = 4 bytes
		1 0 = 8 bytes
		1 1 = 14 bytes
bit 5 enable 64 byte FIFO (16750 only)
bit 4 reserved
bit 3 dma mode select change status of rxrdy and txrdy pins froom mode 1 to mode 2
bit 2 clear transmit fifo
bit 1 clear receive fifo
bit 0 enable fifo's
*/

/*
line control register (LCR)
LINE_CONTROL
bit 7 divisor latch access bit
bit 6 set break enable	//transmit pin goes into a spacing state when enabled
bit 3 4 5 parity select
		0 x x = no parity
		1 0 0 = odd parity
		1 1 0 = even parity
		1 0 1 = high parity (sticky) - parity bit alwiys high
		1 1 1 = low parity (sticky) - parity bit always low
bit 2 length of stop bit
		0 = one stop bit
		1 = 2 stop bits for words of length 6,7,8 bits
				1.5 stop bits for word lengths of 5 bits
bit 0 1 word length
		0 0 = 5 bits
		1 0 = 6 bits
		0 1 = 7 bits
		1 1 = 8 bits
*/

/*
modem control register (MCR)
MODEM_CONTROL
bit 7 reserved
bit 6 reserved
bit 5 autoflow control enabled (16750 only)
bit 4 loopback mode
bit 3 aux output 2
bit 2 aux output 1
bit 1 force request to send
bit 0 force data terminal ready
aux 1 could be a midi 4MHz crystal controller
*/

/*
line status register (LSR)
LINE_STATUS
bit 7 error in received FIFO
bit 6 empty data holding registers - should be no activity on the transmit line
bit 5 empty transmitter holding register - data can be sent, but there might be a byte being sent out currently
bit 4 break interrupt - received data line is held at 0 longer than it takes to send a full word
		this includes start bit, data bits, parity bits, and stop bits
bit 3 framing error - last bit is not a stop bit
bit 2 parity error
bit 1 overrun error - not reading fast enough from the port
bit 0 data ready
a two byte buffer holds data as it is shifted out
*/

/*
modem status register (MSR)
MODEM_STATUS
bit 7 carrier detect
bit 6 ring indicator
bit 5 data set ready
bit 4 clear to send
bit 3 delta data carrier detect - change in data carrier detect line since this was last read
bit 2 trailing edge ring indicator - ring indicator line went from low to high
bit 1 delta data set ready - change in data set ready line since this was last read
bit 0 delta clear to send - change in clear to send line since this was last read
*/

//this driver will be interrupt driven
	//i think it works best for what I want to do with it (to include gdb debugging)
//int 3 and 4 are used for the serial ports
//int 4 = com 1 3
//int 3 = com 2 4

