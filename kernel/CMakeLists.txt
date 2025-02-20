cmake_minimum_required(VERSION 3.22)
project(doors-os)

add_custom_target(
    kernel64
    BYPRODUCTS ./kernel64.debug ./kernel64 ./target ./kerneltest64
    COMMAND cargo build --release --target x86_64-unknown-none
    COMMAND mkdir -p ./build
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./kernel64.debug
    COMMAND cp -u target/x86_64-unknown-none/release/kernel ./kernel64
    COMMAND strip ./kernel64
    COMMAND cp -u target/x86_64-unknown-none/release/kerneltest ./kerneltest64
    COMMAND strip ./kerneltest64
)

add_custom_target(
    disassemble
    BYPRODUCTS ./disassemble.txt
    COMMAND cargo objdump --release --bin kernel -q -- -d > disassemble.txt
)

file(GENERATE OUTPUT ./bochs_symbol_filter INPUT ./bsfcontents FILE_PERMISSIONS OWNER_READ OWNER_WRITE OWNER_EXECUTE)

add_custom_target(
    symbols_bochs
    DEPENDS kernel64
    BYPRODUCTS ./symbols_bochs ./kernel64.sym
    COMMAND cargo objdump -q --bin kernel --release -- -t > ./objdump_output
    COMMAND tail -n +5 objdump_output > objdump_output2
    COMMAND ./bochs_symbol_filter
)
