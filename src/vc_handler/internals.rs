#[macro_export]
macro_rules! make_vmm_handler {
    ($target_name:ident, $inner_handler:ident) => {
        #[unsafe(naked)]
        pub extern "x86-interrupt" fn $target_name(
            _stack_frame: ExceptionStackFrame,
            _code: u64
        ) {
            // https://github.com/llvm/llvm-project/issues/10965
            // 1. push all registers
            // 2. call the real function using normal conventions, with a ptr to the stack structure
            // 3. upon return, re-set the values from the stack structure (they can be modified!)
            naked_asm!(
                // Save general purpose registers
                "push rbp",

                // Scratch registers (x64)
                "push r15",
                "push r14",
                "push r13",
                "push r12",
                "push r11",
                "push r10",
                "push r9",
                "push r8",

                // Scratch registers (x86)
                "push rdi",
                "push rsi",

                "push rdx",
                "push rcx",
                "push rbx",
                "push rax",

                // Provide stack address in the argument register
                "mov rdi, rsp",

                // Call the real handler
                "sub rsp, 0x20",
                "cld",
                "call {}",
                "add rsp, 0x20",

                // Restore registers
                // Scratch registers (x86)
                "pop rax",
                "pop rbx",
                "pop rcx",
                "pop rdx",

                "pop rsi",
                "pop rdi",

                "pop r8",
                "pop r9",
                "pop r10",
                "pop r11",
                "pop r12",
                "pop r13",
                "pop r14",
                "pop r15",
                "pop rbp",

                // Skip error code! iretq expects the error code to have been popped
                "add rsp, 0x8",

                // Call iret
                "iretq",
                sym $inner_handler
            )
        }
    }
}