#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif

#define COM1_PORT		0x3F8   /* COM1 */

extern unsigned long PAGE_SIZE;

EXTERNC void init();
EXTERNC char read_serial();
EXTERNC void write_serial(char a);
EXTERNC void initialize(unsigned int base_port, int line_control, int fifo, unsigned int speed, int modem_control, int enable_ints);
