use crate::x86;
use crate::X86;

// winapi is stdcall, which means args are right to left and callee-cleaned.
// The caller of winapi functions is responsible for pushing/popping the
// return address, because some callers actually 'jmp' directly.

// For now, a magic variable that makes it easier to spot.
pub const STDOUT_HFILE: u32 = 0xF11E_0100;

#[allow(non_snake_case)]
pub mod kernel32 {
    use super::*;
    use tsify::Tsify;

    #[derive(Debug, tsify::Tsify, serde::Serialize)]
    pub struct Mapping {
        pub addr: u32,
        pub size: u32,
        pub desc: String,
    }

    pub struct State {
        // Address image was loaded at.
        pub image_base: u32,
        // Address of TEB (FS register).
        pub teb: u32,
        pub mappings: Vec<Mapping>,
    }
    impl State {
        pub fn new() -> Self {
            let mappings = vec![Mapping {
                addr: 0,
                size: x86::NULL_POINTER_REGION_SIZE,
                desc: "avoid null pointers".into(),
            }];
            State {
                image_base: 0,
                teb: 0,
                mappings,
            }
        }

        pub fn add_mapping(&mut self, mapping: Mapping) {
            let pos = self
                .mappings
                .iter()
                .position(|m| m.addr > mapping.addr)
                .unwrap_or(self.mappings.len());
            if pos > 0 {
                let prev = &self.mappings[pos - 1];
                assert!(prev.addr + prev.size <= mapping.addr);
            }
            if pos < self.mappings.len() {
                let next = &self.mappings[pos];
                assert!(mapping.addr + mapping.size <= next.addr);
            }
            self.mappings.insert(pos, mapping);
        }

        pub fn alloc(&mut self, size: u32, desc: String) -> &Mapping {
            let mut end = 0;
            for (pos, mapping) in self.mappings.iter().enumerate() {
                let space = mapping.addr - end;
                if space > size {
                    self.mappings.insert(
                        pos,
                        Mapping {
                            addr: end,
                            size,
                            desc,
                        },
                    );
                    return &self.mappings[pos];
                }
                end = mapping.addr + mapping.size + (0x1000 - 1) & !(0x1000 - 1);
            }
            panic!("alloc of {size:x} failed");
        }
    }

    pub fn ExitProcess(x86: &mut X86) {
        let uExitCode = x86.pop();
        x86.host.exit(uExitCode);
    }

    pub fn GetModuleHandleA(x86: &mut X86) {
        let lpModuleName = x86.pop();
        if lpModuleName != 0 {
            log::error!("unimplemented: GetModuleHandle(non-null)")
        }
        // HMODULE is base address of current module.
        x86.regs.eax = x86.state.kernel32.image_base;
    }

    pub fn WriteFile(x86: &mut X86) {
        let hFile = x86.pop();
        let lpBuffer = x86.pop();
        let nNumberOfBytesToWrite = x86.pop();
        let lpNumberOfBytesWritten = x86.pop();
        let lpOverlapped = x86.pop();

        assert!(hFile == STDOUT_HFILE);
        assert!(lpOverlapped == 0);
        let buf = &x86.mem[lpBuffer as usize..(lpBuffer + nNumberOfBytesToWrite) as usize];

        let n = x86.host.write(buf);

        x86.write_u32(lpNumberOfBytesWritten, n as u32);
        x86.regs.eax = 1;
    }

    pub fn VirtualAlloc(x86: &mut X86) {
        let lpAddress = x86.pop();
        let dwSize = x86.pop();
        let flAllocationType = x86.pop();
        let _flProtec = x86.pop();

        assert!(lpAddress == 0);
        // TODO round dwSize to page boundary
        assert!(flAllocationType == 0x1000); // MEM_COMMIT

        let mapping = x86.state.kernel32.alloc(dwSize, "VirtualAlloc".into());
        x86.regs.eax = mapping.addr;
    }
}

#[allow(non_snake_case)]
mod user32 {
    use super::*;
    pub fn RegisterClassA(x86: &mut X86) {
        let lpWndClass = x86.pop();
        log::warn!("todo: RegisterClassA({:x})", lpWndClass);
    }
    pub fn CreateWindowExA(x86: &mut X86) {
        let dwExStyle = x86.pop();
        let lpClassName = x86.pop();
        let lpWindowName = x86.pop();
        let dwStyle = x86.pop();
        let X = x86.pop();
        let Y = x86.pop();
        let nWidth = x86.pop();
        let nHeight = x86.pop();
        let hWndParent = x86.pop();
        let hMenu = x86.pop();
        let hInstance = x86.pop();
        let lpParam = x86.pop();
        log::warn!("todo: CreateWindowExA({dwExStyle:x}, {lpClassName:x}, {lpWindowName:x}, {dwStyle:x}, {X:x}, {Y:x}, {nWidth:x}, {nHeight:x}, {hWndParent:x}, {hMenu:x}, {hInstance:x}, {lpParam:x})");
    }
    pub fn UpdateWindow(x86: &mut X86) {
        let hWnd = x86.pop();
        log::warn!("todo: UpdateWindow({hWnd:x})");
    }
}

pub fn resolve(sym: &str) -> Option<fn(&mut X86)> {
    Some(match sym {
        "kernel32.dll!ExitProcess" => kernel32::ExitProcess,
        "kernel32.dll!GetModuleHandleA" => kernel32::GetModuleHandleA,
        "kernel32.dll!WriteFile" => kernel32::WriteFile,
        "kernel32.dll!VirtualAlloc" => kernel32::VirtualAlloc,
        "user32.dll!RegisterClassA" => user32::RegisterClassA,
        "user32.dll!CreateWindowExA" => user32::CreateWindowExA,
        "user32.dll!UpdateWindow" => user32::UpdateWindow,
        _ => return None,
    })
}

pub struct State {
    pub kernel32: kernel32::State,
}
impl State {
    pub fn new() -> Self {
        State {
            kernel32: kernel32::State::new(),
        }
    }
}
