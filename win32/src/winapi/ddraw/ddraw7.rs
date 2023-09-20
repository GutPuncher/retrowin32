//! Implementation of DirectDraw7 interfaces.

use super::{types::*, IDirectDrawPalette, State, DDERR_GENERIC, DD_OK};
use crate::{
    winapi::{ddraw, types::*, vtable},
    Machine,
};
use bitflags::bitflags;
use memory::Pod;

const TRACE_CONTEXT: &'static str = "ddraw/7";

pub const IID_IDirectDraw7: [u8; 16] = [
    0xc0, 0x5e, 0xe6, 0x15, 0x9c, 0x3b, 0xd2, 0x11, 0xb9, 0x2f, 0x00, 0x60, 0x97, 0x97, 0xea, 0x5b,
];

#[win32_derive::shims_from_x86]
pub(super) mod IDirectDraw7 {
    use super::*;

    vtable![IDirectDraw7 shims
        QueryInterface todo,
        AddRef todo,
        Release ok,
        Compact todo,
        CreateClipper todo,
        CreatePalette ok,
        CreateSurface ok,
        DuplicateSurface todo,
        EnumDisplayModes ok,
        EnumSurfaces todo,
        FlipToGDISurface todo,
        GetCaps todo,
        GetDisplayMode todo,
        GetFourCCCodes todo,
        GetGDISurface todo,
        GetMonitorFrequency todo,
        GetScanLine todo,
        GetVerticalBlankStatus todo,
        Initialize todo,
        RestoreDisplayMode todo,
        SetCooperativeLevel ok,
        SetDisplayMode ok,
        WaitForVerticalBlank todo,
        GetAvailableVidMem todo,
        GetSurfaceFromDC todo,
        RestoreAllSurfaces todo,
        TestCooperativeLevel todo,
        GetDeviceIdentifier todo,
        StartModeTest todo,
        EvaluateMode todo,
    ];

    #[win32_derive::dllexport]
    fn Release(_machine: &mut Machine, this: u32) -> u32 {
        log::warn!("{this:x}->Release()");
        0 // TODO: return refcount?
    }

    #[win32_derive::dllexport]
    fn CreatePalette(
        machine: &mut Machine,
        this: u32,
        flags: Result<DDPCAPS, u32>,
        entries: u32,
        lplpPalette: u32,
        unused: u32,
    ) -> u32 {
        let flags = flags.unwrap();
        if !flags.contains(DDPCAPS::_8BIT) {
            todo!();
        }
        // TODO: if palette is DDPCAPS_8BITENTRIES then SetEntries needs change too.

        let palette = IDirectDrawPalette::new(machine);
        let entries = machine
            .mem()
            .view_n::<PALETTEENTRY>(entries, 256)
            .to_vec()
            .into_boxed_slice();
        machine.state.ddraw.palettes.insert(palette, entries);
        machine.mem().put::<u32>(lplpPalette, palette);
        DD_OK
    }

    #[win32_derive::dllexport]
    fn CreateSurface(
        machine: &mut Machine,
        this: u32,
        desc: Option<&DDSURFACEDESC2>,
        lpDirectDrawSurface7: Option<&mut u32>,
        unused: u32,
    ) -> u32 {
        let desc = desc.unwrap();
        assert!(std::mem::size_of::<DDSURFACEDESC2>() == desc.dwSize as usize);
        let lpDirectDrawSurface7 = lpDirectDrawSurface7.unwrap();

        let mut opts = crate::host::SurfaceOptions::default();
        if desc.dwFlags.contains(DDSD::WIDTH) {
            opts.width = desc.dwWidth;
        }
        if desc.dwFlags.contains(DDSD::HEIGHT) {
            opts.height = desc.dwHeight;
        }
        if let Some(caps) = desc.caps() {
            log::warn!("  caps: {:?}", caps.caps1());
            if caps.caps1().contains(DDSCAPS::PRIMARYSURFACE) {
                opts.width = machine.state.ddraw.width;
                opts.height = machine.state.ddraw.height;
                opts.primary = true;
            }
        }

        if let Some(count) = desc.back_buffer_count() {
            log::warn!("  back_buffer: {count:x}");
        }

        //let window = machine.state.user32.get_window(machine.state.ddraw.hwnd);
        let surface = machine.host.create_surface(&opts);

        let x86_surface = IDirectDrawSurface7::new(machine);
        *lpDirectDrawSurface7 = x86_surface;
        machine.state.ddraw.surfaces.insert(
            x86_surface,
            ddraw::Surface {
                host: surface,
                width: opts.width,
                height: opts.height,
                palette: 0,
                pixels: 0,
            },
        );

        DD_OK
    }

    #[win32_derive::dllexport]
    async fn EnumDisplayModes(
        machine: &mut Machine,
        this: u32,
        dwFlags: u32,
        lpSurfaceDesc: Option<&DDSURFACEDESC2>,
        lpContext: u32,
        lpEnumCallback: u32,
    ) -> u32 {
        if lpSurfaceDesc.is_some() {
            todo!()
        }

        let mem = machine.memory.mem();
        let desc_addr = machine
            .state
            .ddraw
            .heap
            .alloc(mem, std::mem::size_of::<DDSURFACEDESC2>() as u32);
        let desc = mem.view_mut::<DDSURFACEDESC2>(desc_addr);
        unsafe { desc.clear_struct() };
        // TODO: offer multiple display modes rather than hardcoding this one.
        desc.dwSize = std::mem::size_of::<DDSURFACEDESC2>() as u32;
        desc.dwWidth = 320;
        desc.dwHeight = 200;
        desc.ddpfPixelFormat = DDPIXELFORMAT {
            dwSize: std::mem::size_of::<DDPIXELFORMAT>() as u32,
            dwFlags: 0,
            dwFourCC: 0,
            dwRGBBitCount: 8,
            dwRBitMask: 0xFF000000,
            dwGBitMask: 0x00FF0000,
            dwBBitMask: 0x0000FF00,
            dwRGBAlphaBitMask: 0x000000FF,
        };

        crate::shims::call_x86(machine, lpEnumCallback, vec![desc_addr, lpContext]).await;

        machine
            .state
            .ddraw
            .heap
            .free(machine.memory.mem(), desc_addr);

        DD_OK
    }

    bitflags! {
        pub struct DDSCL: u32 {
            const DDSCL_FULLSCREEN = 0x0001;
            const DDSCL_ALLOWREBOOT = 0x0002;
            const DDSCL_NOWINDOWCHANGES = 0x0004;
            const DDSCL_NORMAL = 0x0008;
            const DDSCL_EXCLUSIVE = 0x0010;
            const DDSCL_ALLOWMODEX = 0x0040;
            const DDSCL_SETFOCUSWINDOW = 0x0080;
            const DDSCL_SETDEVICEWINDOW = 0x0100;
            const DDSCL_CREATEDEVICEWINDOW = 0x0200;
            const DDSCL_MULTITHREADED = 0x0400;
            const DDSCL_FPUSETUP = 0x0800;
            const DDSCL_FPUPRESERVE =  0x1000;
        }
    }
    impl TryFrom<u32> for DDSCL {
        type Error = u32;

        fn try_from(value: u32) -> Result<Self, Self::Error> {
            DDSCL::from_bits(value).ok_or(value)
        }
    }

    #[win32_derive::dllexport]
    pub fn SetCooperativeLevel(
        machine: &mut Machine,
        this: u32,
        hwnd: HWND,
        flags: Result<DDSCL, u32>,
    ) -> u32 {
        // TODO: this triggers behaviors like fullscreen.
        machine.state.ddraw.hwnd = hwnd;
        DD_OK
    }

    #[win32_derive::dllexport]
    pub fn SetDisplayMode(
        machine: &mut Machine,
        this: u32,
        width: u32,
        height: u32,
        bpp: u32,
        refresh: u32,
        flags: u32,
    ) -> u32 {
        machine.state.ddraw.width = width;
        machine.state.ddraw.height = height;
        if !machine.state.ddraw.hwnd.is_null() {
            machine
                .state
                .user32
                .get_window(machine.state.ddraw.hwnd)
                .host
                .set_size(width, height);
        }
        DD_OK
    }
}

#[win32_derive::shims_from_x86]
pub(super) mod IDirectDrawSurface7 {
    use super::*;

    vtable![IDirectDrawSurface7 shims
        QueryInterface todo,
        AddRef todo,
        Release ok,
        AddAttachedSurface todo,
        AddOverlayDirtyRect todo,
        Blt todo,
        BltBatch todo,
        BltFast ok,
        DeleteAttachedSurface todo,
        EnumAttachedSurfaces todo,
        EnumOverlayZOrders todo,
        Flip ok,
        GetAttachedSurface ok,
        GetBltStatus todo,
        GetCaps todo,
        GetClipper todo,
        GetColorKey todo,
        GetDC ok,
        GetFlipStatus todo,
        GetOverlayPosition todo,
        GetPalette todo,
        GetPixelFormat todo,
        GetSurfaceDesc ok,
        Initialize todo,
        IsLost todo,
        Lock ok,
        ReleaseDC ok,
        Restore ok,
        SetClipper todo,
        SetColorKey todo,
        SetOverlayPosition todo,
        SetPalette ok,
        Unlock ok,
        UpdateOverlay todo,
        UpdateOverlayDisplay todo,
        UpdateOverlayZOrder todo,
        GetDDInterface todo,
        PageLock todo,
        PageUnlock todo,
        SetSurfaceDesc todo,
        SetPrivateData todo,
        GetPrivateData todo,
        FreePrivateData todo,
        GetUniquenessValue todo,
        ChangeUniquenessValue todo,
        SetPriority todo,
        GetPriority todo,
        SetLOD todo,
        GetLOD todo,
    ];

    pub fn new(machine: &mut Machine) -> u32 {
        let ddraw = &mut machine.state.ddraw;
        let lpDirectDrawSurface7 = ddraw.heap.alloc(machine.memory.mem(), 4);
        let vtable = ddraw.vtable_IDirectDrawSurface7;
        machine.mem().put::<u32>(lpDirectDrawSurface7, vtable);
        lpDirectDrawSurface7
    }

    #[win32_derive::dllexport]
    fn Release(_machine: &mut Machine, this: u32) -> u32 {
        log::warn!("{this:x}->Release()");
        0 // TODO: return refcount?
    }

    #[win32_derive::dllexport]
    fn BltFast(
        machine: &mut Machine,
        this: u32,
        x: u32,
        y: u32,
        lpSurf: u32,
        lpRect: Option<&RECT>,
        flags: u32,
    ) -> u32 {
        if flags != 0 {
            log::warn!("BltFlat flags: {:x}", flags);
        }
        let (dst, src) = unsafe {
            let dst = machine.state.ddraw.surfaces.get_mut(&this).unwrap() as *mut ddraw::Surface;
            let src = machine.state.ddraw.surfaces.get(&lpSurf).unwrap() as *const ddraw::Surface;
            assert_ne!(dst as *const ddraw::Surface, src);
            (&mut *dst, &*src)
        };
        let rect = lpRect.unwrap();
        let sx = rect.left;
        let w = rect.right - sx;
        let sy = rect.top;
        let h = rect.bottom - sy;
        dst.host.bit_blt(x, y, src.host.as_ref(), sx, sy, w, h);
        DD_OK
    }

    bitflags! {
        pub struct DDFLIP: u32 {
            const DDFLIP_WAIT = 0x00000001;
            const DDFLIP_EVEN = 0x00000002;
            const DDFLIP_ODD = 0x00000004;
            const DDFLIP_NOVSYNC = 0x00000008;
            const DDFLIP_STEREO = 0x00000010;
            const DDFLIP_DONOTWAIT= 0x00000020;
            const DDFLIP_INTERVAL2= 0x02000000;
            const DDFLIP_INTERVAL3= 0x03000000;
            const DDFLIP_INTERVAL4= 0x04000000;
        }
    }
    impl TryFrom<u32> for DDFLIP {
        type Error = u32;

        fn try_from(value: u32) -> Result<Self, Self::Error> {
            DDFLIP::from_bits(value).ok_or(value)
        }
    }

    #[win32_derive::dllexport]
    fn Flip(machine: &mut Machine, this: u32, lpSurf: u32, flags: Result<DDFLIP, u32>) -> u32 {
        let surface = machine.state.ddraw.surfaces.get_mut(&this).unwrap();
        surface.host.flip();
        DD_OK
    }

    #[win32_derive::dllexport]
    fn GetAttachedSurface(
        machine: &mut Machine,
        this: u32,
        _lpDDSCaps2: u32,
        lpDirectDrawSurface7: u32,
    ) -> u32 {
        // TODO: consider caps.
        // log::warn!("{this:x}->GetAttachedSurface({lpDDSCaps2:x}, {lpDirectDrawSurface7:x})");
        let this_surface = machine.state.ddraw.surfaces.get(&this).unwrap();
        let host = this_surface.host.get_attached();

        let surface = ddraw::Surface {
            host,
            width: this_surface.width,
            height: this_surface.height,
            palette: this_surface.palette,
            pixels: this_surface.pixels,
        };
        let x86_surface = new(machine);

        machine.mem().put::<u32>(lpDirectDrawSurface7, x86_surface);
        machine.state.ddraw.surfaces.insert(x86_surface, surface);
        DD_OK
    }

    #[win32_derive::dllexport]
    fn GetDC(machine: &mut Machine, this: u32, lpHDC: u32) -> u32 {
        let mut dc = crate::winapi::gdi32::DC::new();
        dc.ddraw_surface = this;
        let handle = machine.state.gdi32.dcs.add(dc);
        machine.mem().put::<u32>(lpHDC, handle);
        DD_OK
    }

    #[win32_derive::dllexport]
    fn GetSurfaceDesc(
        machine: &mut Machine,
        this: u32,
        lpDesc: Option<&mut DDSURFACEDESC2>,
    ) -> u32 {
        let surf = machine.state.ddraw.surfaces.get(&this).unwrap();
        let desc = lpDesc.unwrap();
        assert!(desc.dwSize as usize == std::mem::size_of::<DDSURFACEDESC2>());
        let mut flags = desc.dwFlags;
        if flags.contains(DDSD::WIDTH) {
            desc.dwWidth = surf.width;
            flags.remove(DDSD::WIDTH);
        }
        if flags.contains(DDSD::HEIGHT) {
            desc.dwHeight = surf.height;
            flags.remove(DDSD::HEIGHT);
        }
        if !flags.is_empty() {
            log::warn!(
                "unimp: {:?} for {this:x}->GetSurfaceDesc({desc:?})",
                desc.dwFlags
            );
        }
        DDERR_GENERIC
    }

    #[win32_derive::dllexport]
    pub fn Lock(
        machine: &mut Machine,
        this: u32,
        rect: Option<&RECT>,
        desc: Option<&mut DDSURFACEDESC2>,
        flags: Result<DDLOCK, u32>,
        unused: u32,
    ) -> u32 {
        if rect.is_some() {
            // TODO: once we implement this, we need corresponding logic in Unlock.
            todo!();
        }
        let desc = desc.unwrap();
        let surf = machine.state.ddraw.surfaces.get_mut(&this).unwrap();
        let bytes_per_pixel = 1; // TODO: where does this come from?
        if surf.pixels == 0 {
            surf.pixels = machine.state.ddraw.heap.alloc(
                machine.memory.mem(),
                surf.width * surf.height * bytes_per_pixel,
            );
        }
        desc.dwFlags = DDSD::LPSURFACE;
        desc.lpSurface = surf.pixels;
        desc.lPitch_dwLinearSize = surf.width * bytes_per_pixel;
        DD_OK
    }

    #[win32_derive::dllexport]
    fn ReleaseDC(_machine: &mut Machine, _this: u32, _hDC: u32) -> u32 {
        // leak
        DD_OK
    }

    #[win32_derive::dllexport]
    fn Restore(_machine: &mut Machine, _this: u32) -> u32 {
        DD_OK
    }

    #[win32_derive::dllexport]
    fn SetPalette(machine: &mut Machine, this: u32, palette: u32) -> u32 {
        machine.state.ddraw.surfaces.get_mut(&this).unwrap().palette = palette;
        machine.state.ddraw.palette_hack = palette;
        DD_OK
    }

    #[win32_derive::dllexport]
    pub fn Unlock(machine: &mut Machine, this: u32, rect: Option<&mut RECT>) -> u32 {
        let surf = machine.state.ddraw.surfaces.get_mut(&this).unwrap();
        if let Some(rect) = rect {
            // TODO: needs to match the rect passed in Lock.
            rect.left = 0;
            rect.top = 0;
            rect.right = surf.width;
            rect.bottom = surf.height;
        }
        let phack = machine.state.ddraw.palette_hack;
        if surf.pixels != 0 && phack != 0 {
            let bytes_per_pixel = 1; // TODO: where does this come from?
            let pixels = machine
                .memory
                .mem()
                .view_n::<u8>(surf.pixels, surf.width * surf.height * bytes_per_pixel);
            let palette = machine.state.ddraw.palettes.get(&phack).unwrap();
            // XXX very inefficient
            let pixels32: Vec<_> = pixels
                .iter()
                .map(|&i| {
                    let p = &palette[i as usize];
                    [p.peRed, p.peGreen, p.peBlue, 255]
                })
                .collect();
            surf.host.write_pixels(&pixels32);
        }
        DD_OK
    }
}
