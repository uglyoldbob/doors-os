;compile with the following command line under dos
;NASM -o botstrap.bin botstrap.asm
;this is the FAT12 floppy drive bootsector for the Doors OS 
;byte 0 is the first byte of the file

;we're going to try to use data segment offsets
;(because this makes preserving FAT12 formatting information 100X easier/faster)
;but this will be very interesting
start:
jmp short begin
nop

times 3Bh db 0
begin:
jmp short main
;skip data so we don't go insane or crash the CPU
PreCluster dw 0h					;this stores the number of sectors in the partition before the first cluster 
							;(used when reading kernel)
Cluster dw 0000h						;stores the cluster for the kernel when we read it from disk
Cylinder db 00h						;stores the cylinder value for int 13
Head db 00h							;stores the head value for int 13
Sector db 00h						;stores the sector value for int 13
FileName db 'DOORS   BIN'				;the name of the kernel file
Message db 'Doors loading', 13, 10, 0		;the success message
Oops db 'Oops', 13, 10, 0				;error message
main:
	cli					;disable interupts while we set up a stack
	mov ax, 07C0h
	mov ds, ax	
	mov es, ax				;required for the first call to ReadSectors (where we read in the root directory)
						;the root directory is no longer required after we begin reading/writing the FAT table
	mov ax, 0050h			;the stack goes in the lower portion of conventional memory
	mov ss, ax				;it takes up 1FFh bytes
	mov sp, 0x01FF
	sti
;enable A20
	in	al, 92h
	or	al, 2
	out	92h, al
;check for it
	xor AX, AX
	in AL, 92h
	bt ax, 1
	jc Enabled
	mov si, Oops
	call DisplayMessage
Enabled:
;********************************
;now that we have set up our required stuff for our bootsector to work properly (this has been tested for verification)
;we need to do something useful :)
;find where the root directory begins
;determine size of root directory
;read this into memory at 7C0:0200
;RootDirSectors = ((BPB_RootEntCnt * 32) + (BPB_BytsPerSec - 1)) / BPB_BytsPerSec
; this computation rounds up.
;FirstRootDirSecNum = BPB_ResvdSecCnt + (BPB_NumFATs * BPB_FATSz16);
;we dont need to determine if this is a floppy disk which always uses FAT12
	xor cx, cx			;cx = 0
	xor dx, dx			;so we can multiply
	mov ax, 0020h		;size of a directory entry
	mul WORD [11h]		;multiply by maximum root entries
	div WORD [0Bh]		;sectors used by root directory
	xchg ax, cx	
	mov al, [10h]		;number of FATs (BYTE)
	mul WORD [16h]		;AX * sectors per fat = DX:AX
	add ax, [0Eh]		;add in bootsector sectors
	mov [PreCluster], ax	;store that value
	add [PreCluster], cx	;add in the size of the root directory
	mov bx, 0200h		;right above this bootsector
	call ReadSectors		;read the root directory
;search for the kernel
	mov cx, [11h]		;retrieve max root entries
	mov di, 200h		;locate the first entry
Looping:
	push cx		;save cx, becuase this is a nested loop
	mov cx, 000Bh	;eleven characters in a filename
	mov si, FileName	;the location of the name of the filename (not zero terminated!)
	push di
rep 	cmpsb			;is it a match?
	pop di
	je FatRead		;it is a match
	pop cx		;we are out of the inner loop
	add di, 0020h	;next directory entry
	loop Looping
	mov si, Oops
	call DisplayMessage
	jmp $			;freeze up so se know that we are stupid
FatRead:
;size of fat = sizeof fat [16h]* numFATS[10h]
	mov dx, WORD[di + 001Ah]	;the first cluster of the kernel file
	mov WORD [Cluster], dx
	xor ax, ax
	mov al, BYTE [10h]		;the number of FATs
	mul WORD [16h]			;sectors per FAT
	mov cx, ax
	mov ax, WORD [0Eh]		;reserved sectors (sectors before the first FAT)
	mov bx, 0200h			;the location where we read the FAT to (7C00:200)
	call ReadSectors			;read the sectors into memory
;we have located the bottom of the FAT table in RAM				
;read kernel into memory (090:0000, then "return" to it, transferring control to the kernel
	mov ax, 0x0090          ;destination of image CS
	mov es, ax			;forgot to do this
	mov bx, 00h
	push    bx
Load:
	mov ax, WORD [Cluster]		;cluster to read
	pop bx				;buffer to read to
	call ClusterLBA
	xor cx, cx
	mov cl, BYTE [0Dh]		;sectors per cluster (number of sectors to read)
	call ReadSectors
	push bx
;calculate next cluster
	mov ax, WORD [Cluster]		;what is the current cluster?
	mov cx, ax
	mov dx, ax				;copy that number a couple times
	shr dx, 1h
	add cx, dx				;sum for (3/2)
	mov bx, 200h			;location of FAT in RAM
	add bx, cx				;index into FAT (so we can read the next cluster number and do some stuff to it)
	mov dx, WORD [bx]			;read two bytes from FAT (12 bits requires two bytes to be read and 4 bits thrown away)
	test ax, 0001h			;check for type (even or odd cluster)
	jnz Odd
Even:
	and dx, 0000111111111111b	;we only want the lower twelve bits
	jmp Done
Odd:
	shr dx, 0004h			;take high twelve bits
Done:
	mov WORD [Cluster], dx		;store the new cluster value
	cmp dx, 0FF0h			;test for EOF
	jb Load				;keep reading if not done
	mov si, Message
	call DisplayMessage
	cli
	xor ax, ax
	mov ds, ax              
	lgdt [gdt_desc + 0x7C00]
	mov eax, cr0		;enable pmode
	or al, 1			
	mov cr0, eax
	jmp 0x08:0x0900

[BITS 16]
;ROUTINES GO BELOW HERE
ClusterLBA: 		;converts a FAT cluster number to LBA
	sub ax, 0002h	;cluster number starts at 2
	xor cx, cx		;cx = 0
	mov cl, BYTE [0Dh];sectors per cluster
	mul cx
	add ax, WORD [PreCluster]	;add in sectors before the first cluster
	ret			;go back to where yee came from :)

ReadSectors:			;this reads multiple sectors in a row, one sector at a time
					;ax = begin; es:bx = mem location; cx = how many sectors
					;I choose this method to make reading sectors easier
	push cx			;save number of sectors read (so we can place the kernel file appropiately)
.Main
	mov di, 0x0005		;five tries if there are errors
.SectorLoop
	push ax
	push bx
	push cx
	xor dx, dx			;clear dx for division
	div WORD [18h]		;divide ax (beginning of root in LBA) by bytes per sector
	inc dl			;adjust for sector 0
	mov [Sector], dl		;store that value
	xor dx, dx			;prepare to divide
	div WORD [1Ah]		;divide by the number of heads
	mov [Head], dl		;store the value
	mov [Cylinder], al	;store the value
	mov ah, 02h			;we are reading sectors
	mov al, 01h			;read one sector
	mov ch, BYTE [Cylinder]	;restore the track value
	mov cl, BYTE [Sector]	;restore the sector value
	mov dh, BYTE [Head]	;restore the value
	mov dl, BYTE [24h]	;retrieve the drive number as recognized by the BIOS int13 routine
	int 13h			;read one sector
	jnc .Success
	xor ax, ax			;reset disk
	int 13h			;call BIOS
	dec di			;decrement error counter
	pop cx
	pop bx
	pop ax
	jnz .SectorLoop		;read again if there was an error
;	int 18h
.Success
	pop cx
	pop bx
	pop ax
	add bx, [0Bh]		;add in bytes per sector for the next buffer zone
	inc ax			;next sector to read
	loop .Main
	pop cx			;restore number of sectors read
	ret

DisplayMessage:
	lodsb                                       ; load next character
	or      al, al                              ; test for NUL character
	jz      .DONE
	mov     ah, 0x0E                            ; BIOS teletype
	mov     bh, 0x00                            ; display page 0
	mov     bl, 0x07                            ; text attribute
	int     0x10                                ; invoke BIOS
	jmp     DisplayMessage
.DONE:
	ret
gdt:                    ; Address for the GDT
gdt_null:               ; Null Segment
	dd 0
	dd 0
gdt_code:               ; Code segment, read/execute, nonconforming
	dw 0FFFFh
	dw 0x0000
	db 0
	db 10011010b
	db 11001111b
	db 0
gdt_data:               ; Data segment, read/write
	dw 0xFFFF
	dw 0x0000
	db 0
	db 10010010b
	db 11001111b
	db 0
gdt_end:				; Used to calculate the size of the GDT
gdt_desc:				; The GDT descriptor
	dw gdt_end - gdt - 1	; Limit (size)
	dd gdt + 0x7C00		; Address of the GDT
times 510-($-$$) db '?'	; Fill the rest of the sector with zeros (disable to see how much space is left)
dw 0xAA55		; Boot signature