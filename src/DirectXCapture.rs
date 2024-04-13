use winapi::shared::dxgi::{DXGI_MAP_READ, IDXGIAdapter, IDXGIOutput, CreateDXGIFactory1, IID_IDXGISurface, IID_IDXGIFactory1, IDXGIFactory1, IDXGIAdapter1, DXGI_OUTPUT_DESC, IDXGISurface, IDXGIResource, DXGI_RESOURCE_PRIORITY_MAXIMUM, DXGI_MAPPED_RECT};
use winapi::um::d3d11::{D3D11CreateDevice, ID3D11Resource, IID_ID3D11Texture2D, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_SDK_VERSION, D3D11_USAGE_STAGING, D3D11_CPU_ACCESS_READ};
use winapi::shared::dxgi1_2::{IID_IDXGIOutput1, IDXGIOutput1, IDXGIOutputDuplication};
use winapi::um::d3dcommon::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_9_1};
use winapi::shared::minwindef::{UINT, TRUE};
use winapi::um::winnt::{LONG, HRESULT};

use winapi::shared::dxgitype::DXGI_MODE_ROTATION;
use winapi::um::unknwnbase::IUnknown;
use winapi::shared::winerror::S_OK;

use std::io::ErrorKind::{WouldBlock, TimedOut, NotFound};
use std::{io, mem, ptr, slice, thread, ops};
use std::mem::MaybeUninit;

pub struct Capturer {
    device: *mut ID3D11Device,
    context: *mut ID3D11DeviceContext,
    duplication: *mut IDXGIOutputDuplication,
    fastlane: bool, surface: *mut IDXGISurface,
    data: *mut u8, len: usize,
    height: usize,
}

impl Capturer {
    pub fn new(display: &mut Display) -> io::Result<Capturer> {
        let mut device = ptr::null_mut();
        let mut context = ptr::null_mut();
        let mut duplication = ptr::null_mut();
        let mut desc = unsafe { mem::uninitialized() };

        if unsafe {
            D3D11CreateDevice(
                display.adapter as *mut IDXGIAdapter,
                D3D_DRIVER_TYPE_UNKNOWN,
                std::ptr::null_mut(), // No software rasterizer.
                0, // No device flags.
                std::ptr::null_mut(), // Feature levels.
                0, // Feature levels' length.
                D3D11_SDK_VERSION,
                &mut device,
                &mut D3D_FEATURE_LEVEL_9_1,
                &mut context
            )
        } != S_OK {
            // Unknown error.
            return Err(io::ErrorKind::Other.into());
        }

        let res = wrap_hresult(unsafe {
            (*display.inner).DuplicateOutput(
                device as *mut IUnknown,
                &mut duplication
            )
        });

        if let Err(err) = res {
            unsafe {
                (*device).Release();
                (*context).Release();
            }
            return Err(err);
        }

        unsafe { (*duplication).GetDesc(&mut desc); }

        Ok(unsafe {
            Capturer {
                device, context, duplication,
                fastlane: desc.DesktopImageInSystemMemory == TRUE,
                surface: ptr::null_mut(),
                height: (display.height() as usize) / 2 + 2,
                data: ptr::null_mut(),
                len: 0
            }
        })
    }

    unsafe fn load_frame(&mut self, timeout: UINT) -> io::Result<()> {
        let mut frame = ptr::null_mut();
        let mut info = mem::uninitialized();
        self.data = ptr::null_mut();

        wrap_hresult((*self.duplication).AcquireNextFrame(
            timeout,
            &mut info,
            &mut frame
        ))?;

        if self.fastlane {
            let mut rect: MaybeUninit<DXGI_MAPPED_RECT> = mem::MaybeUninit::<DXGI_MAPPED_RECT>::uninit();
            let res = wrap_hresult(
                (*self.duplication).MapDesktopSurface(rect.as_mut_ptr())
            );

            let mut rect = rect.assume_init();
            (*frame).Release();

            if let Err(err) = res {
                Err(err)
            } else {
                self.data = rect.pBits;
                self.len = self.height * rect.Pitch as usize;
                Ok(())
            }
        } else {
            self.ohgodwhat(frame);

            let mut rect = mem::uninitialized();
            wrap_hresult((*self.surface).Map(
                &mut rect,
                DXGI_MAP_READ
            ))?;

            self.data = rect.pBits;
            self.len = self.height * rect.Pitch as usize;
            Ok(())
        }
    }

    unsafe fn ohgodwhat(&mut self, frame: *mut IDXGIResource) -> io::Result<()> {
        self.surface = ptr::null_mut();

        let mut texture: *mut ID3D11Texture2D = ptr::null_mut();
        (*frame).QueryInterface(
            &IID_ID3D11Texture2D,
            &mut texture as *mut *mut _ as *mut *mut _
        );

        let mut texture_desc = mem::uninitialized();
        (*texture).GetDesc(&mut texture_desc);

        texture_desc.Usage = D3D11_USAGE_STAGING;
        texture_desc.BindFlags = 0;
        texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ;
        texture_desc.MiscFlags = 0;

        let mut readable = std::ptr::null_mut();
        let res = wrap_hresult((*self.device).CreateTexture2D(
            &mut texture_desc,
            ptr::null(),
            &mut readable
        ));

        if let Err(err) = res {
            (*frame).Release();
            (*texture).Release();
            (*readable).Release();
            Err(err)
        } else {
            (*readable).SetEvictionPriority(DXGI_RESOURCE_PRIORITY_MAXIMUM);

            let mut surface = std::ptr::null_mut();
            (*readable).QueryInterface(
                &IID_IDXGISurface,
                &mut surface as *mut *mut _ as *mut *mut _
            );

            (*self.context).CopyResource(
                readable as *mut ID3D11Resource,
                texture as *mut ID3D11Resource
            );

            (*frame).Release();
            (*texture).Release();
            (*readable).Release();
            self.surface = surface;
            Ok(())
        }
    }

    pub fn frame<'a>(&'a mut self, timeout: UINT) -> io::Result<&'a [u8]> {
        unsafe {
            (*self.duplication).ReleaseFrame();
            // Release last frame.
            // No error checking needed because we don't care.
            // None of the errors crash anyway.

            if self.fastlane {
                (*self.duplication).UnMapDesktopSurface();
            } else {
                if !self.surface.is_null() {
                    (*self.surface).Unmap();
                    (*self.surface).Release();
                    self.surface = ptr::null_mut();
                }
            }

            // Get next frame.
            self.load_frame(timeout)?;
            Ok(slice::from_raw_parts(self.data, self.len))
        }
    }
}

impl Drop for Capturer {
    fn drop(&mut self) {
        unsafe {
            (*self.duplication).Release();
            (*self.device).Release();
            (*self.context).Release();
        }
    }
}

pub struct Displays {
    factory: *mut IDXGIFactory1,
    adapter: *mut IDXGIAdapter1,
    /// Index of the CURRENT adapter.
    nadapter: UINT,
    /// Index of the NEXT display to fetch.
    ndisplay: UINT
}

impl Displays {
    pub unsafe fn new() -> io::Result<Displays> {
        let mut factory: *mut IDXGIFactory1 = std::ptr::null_mut();
        wrap_hresult(unsafe {
            CreateDXGIFactory1(&IID_IDXGIFactory1, &mut factory as *mut *mut _ as *mut *mut _)
        })?;

        let mut adapter: *mut IDXGIAdapter1 = ptr::null_mut();
        unsafe {
            // On error, our adapter is null, so it's fine.
            (*factory).EnumAdapters(0, &mut adapter as *mut *mut _ as *mut *mut _);
        };

        Ok(Displays {
            factory,
            adapter,
            nadapter: 0,
            ndisplay: 0
        })
    }

    // No Adapter => Some(None)
    // Non-Empty Adapter => Some(Some(OUTPUT))
    // End of Adapter => None
    fn read_and_invalidate(&mut self) -> Option<Option<Display>> {
        // If there is no adapter, there is nothing left for us to do.

        if self.adapter.is_null() {
            return Some(None);
        }

        // Otherwise, we get the next output of the current adapter.

        let output = unsafe {
            let mut output = ptr::null_mut();
            (*self.adapter).EnumOutputs(self.ndisplay, &mut output);
            output
        };

        // If the current adapter is done, we free it.
        // We return None so the caller gets the next adapter and tries again.

        if output.is_null() {
            unsafe {
                (*self.adapter).Release();
                self.adapter = ptr::null_mut();
            }
            return None;
        }

        // Advance to the next display.

        self.ndisplay += 1;

        // We get the display's details.

        let desc = unsafe {
            let mut desc = mem::uninitialized();
            (*output).GetDesc(&mut desc);
            desc
        };

        // We cast it up to the version needed for desktop duplication.

        let mut inner: *mut IDXGIOutput1 = ptr::null_mut();
        unsafe {
            (*output).QueryInterface(
                &IID_IDXGIOutput1,
                &mut inner as *mut *mut _ as *mut *mut _
            );
            (*output).Release();
        }

        // If it's null, we have an error.
        // So we act like the adapter is done.

        if inner.is_null() {
            unsafe {
                (*self.adapter).Release();
                self.adapter = ptr::null_mut();
            }
            return None;
        }

        unsafe {
            (*self.adapter).AddRef();
        }

        Some(Some(Display { inner, adapter: self.adapter, desc }))
    }
}

impl Iterator for Displays {
    type Item = Display;
    fn next(&mut self) -> Option<Display> {
        if let Some(res) = self.read_and_invalidate() {
            res
        } else {
            // We need to replace the adapter.

            self.ndisplay = 0;
            self.nadapter += 1;

            self.adapter = unsafe {
                let mut adapter = ptr::null_mut();
                (*self.factory).EnumAdapters1(
                    self.nadapter,
                    &mut adapter
                );
                adapter
            };

            if let Some(res) = self.read_and_invalidate() {
                res
            } else {
                // All subsequent adapters will also be empty.
                None
            }
        }
    }
}

impl Drop for Displays {
    fn drop(&mut self) {
        unsafe {
            (*self.factory).Release();
            if !self.adapter.is_null() {
                (*self.adapter).Release();
            }
        }
    }
}

pub struct Display {
    inner: *mut IDXGIOutput1,
    adapter: *mut IDXGIAdapter1,
    desc: DXGI_OUTPUT_DESC
}

impl Display {
    pub fn width(&self) -> LONG {
        self.desc.DesktopCoordinates.right -
            self.desc.DesktopCoordinates.left
    }

    pub fn height(&self) -> LONG {
        self.desc.DesktopCoordinates.bottom -
            self.desc.DesktopCoordinates.top
    }

    pub fn rotation(&self) -> DXGI_MODE_ROTATION {
        self.desc.Rotation
    }

    pub fn name(&self) -> &[u16] {
        let s = &self.desc.DeviceName;
        let i = s.iter()
            .position(|&x| x == 0)
            .unwrap_or(s.len());
        &s[..i]
    }
}

impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            (*self.inner).Release();
            (*self.adapter).Release();
        }
    }
}

fn wrap_hresult(x: HRESULT) -> io::Result<()> {
    use std::io::ErrorKind::*;
    Err((match x {
        S_OK => return Ok(()),
        DXGI_ERROR_ACCESS_LOST => ConnectionReset,
        DXGI_ERROR_WAIT_TIMEOUT => TimedOut,
        DXGI_ERROR_INVALID_CALL => InvalidData,
        E_ACCESSDENIED => PermissionDenied,
        DXGI_ERROR_UNSUPPORTED => ConnectionRefused,
        DXGI_ERROR_NOT_CURRENTLY_AVAILABLE => Interrupted,
        DXGI_ERROR_SESSION_DISCONNECTED => ConnectionAborted,
        _ => Other
    }).into())
}

pub struct OutCapturer {
    inner: Capturer,
    width: usize,
    height: usize
}

impl OutCapturer {
    pub fn new(mut display: OutDisplay) -> io::Result<OutCapturer> {
        let width = display.width();
        let height = display.height()/2 + 2;
        let inner = Capturer::new(&mut display.0)?;
        Ok(OutCapturer { inner, width: width.try_into().unwrap(), height: height.try_into().unwrap() })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn frame<'a>(&'a mut self) -> Result<&[u8], io::Error> {
        match self.inner.frame(0) {
            Ok(frame) => Ok(frame),
            Err(ref error) if error.kind() == TimedOut => {
                Err(WouldBlock.into())
            },
            Err(error) => {
                Err(error)
            }
        }
    }
}

pub struct OutDisplay(Display);

impl OutDisplay {
    pub unsafe fn primary() -> io::Result<OutDisplay> {
        match Displays::new()?.next() {
            Some(inner) => Ok(OutDisplay(inner)),
            None => Err(NotFound.into())
        }
    }

    pub unsafe  fn all() -> io::Result<Vec<OutDisplay>> {
        Ok(Displays::new()?
            .map(OutDisplay)
            .collect::<Vec<_>>())
    }

    pub fn width(&self) -> usize {
        self.0.width() as usize
    }

    pub fn height(&self) -> usize {
        self.0.height() as usize
    }
}
