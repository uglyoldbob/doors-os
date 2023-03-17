//serial.h

#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#ifndef _SERIAL_H_
#define _SERIAL_H_

#define COM1_PORT		0x3F8   /* COM1 */

#include "video.h"

EXTERNC void ser_handler();

#ifdef __cplusplus

class serial
{
public:
	serial();
	char read_serial();
	void write_serial(char a);
	void initialize(unsigned int base_port, int line_control, int fifo, unsigned int speed, int modem_control, int enable_ints);
private:
	int is_transmit_empty();
	int serial_received();
	unsigned int port_number;
};
#endif

extern serial kellogs;

//TODO: write functions to finish out the GDB stub
	//so that remote debugging over a serial line can be accomplished


//port offsets from base
#define TRANSMIT_HOLD_BUFFER		0	//w, dlab0
#define RECEIVE_BUFFER					0	//r, dlab0
#define DIVISOR_LATCH_LOW_BYTE	0	//rw, dlab1
#define INT_ENABLE							1 //rw, dlab0, IER
#define DIVISOR_LATCH_HI_BYTE		1 //rw, dlab1
#define INT_ID									2 //r, IIR
#define FIFO_CONTROL						2 //w, FCR
#define LINE_CONTROL						3 //rw, LCR
#define MODEM_CONTROL						4 //rw, MCR
#define LINE_STATUS							5 //r, LSR
#define MODEM_STATUS						6 //r, MSR
#define SCRATCH_REG							7 //rw

//divisor latch access bit
	//set with the line control register
//115,200 maximum bits per second

#define IER_MODEM			0x08
#define IER_LINE_STAT	0x04
#define IER_TRANSMIT	0x02
#define IER_RECEIVED	0x01
#define IER_DEFAULT		IER_RECEIVED | IER_TRANSMIT | IER_LINE_STAT | IER_MODEM

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

#define IID_FIFO_UNAVAIL		0x00
#define IID_FIFO_UNUSABLE		0x80
#define IID_FIFO_USABLE			0xC0	
#define IID_MODEM						0x00
#define IID_TRANSMIT				0x02
#define IID_RECEIVED				0x04
#define IID_REC_STAT				0x06
#define IID_NO_INT					0x01
#define IID_YES_INT					0x00


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
bit 2 1 (
		00 modem status, 
		01 transmitter holding register empty,
		10 received data, 
		11 receiver line status
bit 0 interrupt not pending (0 = interrupt pending)
*/

#define FCR_INT_LVL_1				0x00
#define FCR_INT_LVL_4				0x40
#define FCR_INT_LVL_8				0x80
#define FCR_INT_LVL_14			0xC0
#define FCR_CLEAR_RECEIVE		0x02
#define FCR_CLEAR_SEND			0x04
#define FCR_ENABLE_FIFO			0x01

#define FCR_DEFAULT					FCR_INT_LVL_14 | FCR_ENABLE_FIFO
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

#define LCR_DIVISOR_LATCH				0x80
#define LCR_BREAK_ENABLE 				0x40
#define LCR_NO_PARITY						0x00
#define LCR_ODD_PARITY					0x08
#define LCR_EVEN_PARITY					0x18
#define LCR_HIGH_STICKY_PARITY	0x28
#define LCR_LOW_STICKY_PARITY		0x38
#define LCR_ONE_STOP_BIT				0x00
#define LCR_MORE_STOP_BITS			0x04
#define LCR_WORD_LENGTH_5				0x00
#define LCR_WORD_LENGTH_6				0x01
#define LCR_WORD_LENGTH_7				0x02
#define LCR_WORD_LENGTH_8				0x03
#define LCR_DEFAULT							LCR_WORD_LENGTH_8 | LCR_NO_PARITY | LCR_ONE_STOP_BIT

/*
line control register (LCR_)
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

#define MCR_LOOPBACK		0x10
#define MCR_AUX1				0x08
#define MCR_AUX2				0x04
#define MCR_RTS					0x02
#define MCR_DTR					0x01
#define MCR_DEFAULT			0x00

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

#endif
