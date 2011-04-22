#include "dma.h"

// Quick-access registers and ports for each DMA channel.
unsigned char MaskReg[8]   = { 0x0A, 0x0A, 0x0A, 0x0A, 0xD4, 0xD4, 0xD4, 0xD4 };
unsigned char ModeReg[8]   = { 0x0B, 0x0B, 0x0B, 0x0B, 0xD6, 0xD6, 0xD6, 0xD6 };
unsigned char ClearReg[8]  = { 0x0C, 0x0C, 0x0C, 0x0C, 0xD8, 0xD8, 0xD8, 0xD8 };

unsigned char PagePort[8]  = { 0x87, 0x83, 0x81, 0x82, 0x8F, 0x8B, 0x89, 0x8A };
unsigned char AddrPort[8]  = { 0x00, 0x02, 0x04, 0x06, 0xC0, 0xC4, 0xC8, 0xCC };
unsigned char CountPort[8] = { 0x01, 0x03, 0x05, 0x07, 0xC2, 0xC6, 0xCA, 0xCE };

void startDMA(unsigned int address, unsigned int length, unsigned char channel,
		unsigned char mode)
{	//programs the DMA
	//0x12345 becomes 0x1000:0x2345
	unsigned int page, segment, offset;
  page = address>>16;	//what page is it on (if using 64KB pages)
	offset = address & 0xFFFF; //the offset (0 - FFFF)

	//make sure mode is using the right channel
	mode |= channel;
	//clear interrupts
	asm("cli");
	//mask the DMA so errors wont pop up
	outportb(0x04 | channel, MaskReg[channel]);
	
	//clear any data transfers currently executing (maybe should wait for them to finish?)
	//outportb(0, ClearReg[channel]);

	//send the desired mode to the DMA
	outportb(mode, ModeReg[channel]);

	//initialize the flip-flop
	outportb(0, ClearReg[channel]);

	//send the address for the offset
	outportb(offset & 0xFF, AddrPort[channel]);
	outportb((offset & 0xFF00)>>8, AddrPort[channel]);

	//send the page that the page lies on
	outportb(page, PagePort[channel]);

	//initialize the flip-flop
	outportb(0, ClearReg[channel]);

	//send the length
	outportb(length & 0xFF, CountPort[channel]);
	outportb((length & 0xFF00)>>8, CountPort[channel]);
	
	//enable the DMA now
	outportb(channel, MaskReg[channel]);

	//enable interrupts
	asm("sti");
}
