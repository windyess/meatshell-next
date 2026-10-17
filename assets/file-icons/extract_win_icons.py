# Extract Windows Explorer association icons into assets/file-icons/shell/.
# Layer 1 (preferred) of the SFTP icon stack — see LICENSE.txt.
# Requires Windows + Pillow. Does not run at app runtime.
import ctypes
from ctypes import wintypes
from pathlib import Path

from PIL import Image

shell32 = ctypes.windll.shell32
user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32

SHGFI_ICON = 0x000000100
SHGFI_USEFILEATTRIBUTES = 0x000000010
FILE_ATTRIBUTE_NORMAL = 0x80
FILE_ATTRIBUTE_DIRECTORY = 0x10
DI_NORMAL = 0x0003
BI_RGB = 0
DIB_RGB_COLORS = 0
SIZE = 32

# kind → (probe name, is_directory)
KINDS = {
    "folder": ("folder", True),
    "link": ("file.lnk", False),
    "image": ("file.png", False),
    "video": ("file.mp4", False),
    "audio": ("file.mp3", False),
    "archive": ("file.zip", False),
    "pdf": ("file.pdf", False),
    "code": ("file.py", False),
    "document": ("file.docx", False),
    "table": ("file.xlsx", False),
    "presentation": ("file.pptx", False),
    "csv": ("file.csv", False),
    "html": ("file.html", False),
    "xml": ("file.xml", False),
    "config": ("file.json", False),
    "text": ("file.txt", False),
    "terminal": ("file.bat", False),
    "exe": ("file.exe", False),
    "file": ("file", False),
    "rtf": ("file.rtf", False),
    "font": ("file.ttf", False),
    "email": ("file.eml", False),
    "dll": ("file.dll", False),
    # Dev / Fluent-CDN gaps
    "md": ("file.md", False),
    "toml": ("file.toml", False),
    "rs": ("file.rs", False),
    "go": ("file.go", False),
    "vue": ("file.vue", False),
    "svelte": ("file.svelte", False),
    "wasm": ("file.wasm", False),
    "env": ("file.env", False),
    "gitignore": ("file.gitignore", False),
}


class SHFILEINFOW(ctypes.Structure):
    _fields_ = [
        ("hIcon", wintypes.HICON),
        ("iIcon", ctypes.c_int),
        ("dwAttributes", wintypes.DWORD),
        ("szDisplayName", wintypes.WCHAR * 260),
        ("szTypeName", wintypes.WCHAR * 80),
    ]


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wintypes.DWORD),
        ("biWidth", ctypes.c_long),
        ("biHeight", ctypes.c_long),
        ("biPlanes", wintypes.WORD),
        ("biBitCount", wintypes.WORD),
        ("biCompression", wintypes.DWORD),
        ("biSizeImage", wintypes.DWORD),
        ("biXPelsPerMeter", ctypes.c_long),
        ("biYPelsPerMeter", ctypes.c_long),
        ("biClrUsed", wintypes.DWORD),
        ("biClrImportant", wintypes.DWORD),
    ]


class BITMAPINFO(ctypes.Structure):
    _fields_ = [("bmiHeader", BITMAPINFOHEADER), ("bmiColors", wintypes.DWORD * 3)]


def to_png(hicon: int) -> Image.Image:
    hdc_screen = user32.GetDC(0)
    hdc = gdi32.CreateCompatibleDC(hdc_screen)
    bi = BITMAPINFO()
    bi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.bmiHeader.biWidth = SIZE
    bi.bmiHeader.biHeight = -SIZE
    bi.bmiHeader.biPlanes = 1
    bi.bmiHeader.biBitCount = 32
    bi.bmiHeader.biCompression = BI_RGB
    bits = ctypes.c_void_p()
    hbmp = gdi32.CreateDIBSection(
        hdc, ctypes.byref(bi), DIB_RGB_COLORS, ctypes.byref(bits), None, 0
    )
    old = gdi32.SelectObject(hdc, hbmp)
    user32.DrawIconEx(hdc, 0, 0, hicon, SIZE, SIZE, 0, None, DI_NORMAL)
    gdi32.SelectObject(hdc, old)
    n = SIZE * SIZE * 4
    buf = ctypes.string_at(bits, n)
    rgba = bytearray(n)
    for i in range(0, n, 4):
        rgba[i] = buf[i + 2]
        rgba[i + 1] = buf[i + 1]
        rgba[i + 2] = buf[i]
        rgba[i + 3] = buf[i + 3]
    img = Image.frombytes("RGBA", (SIZE, SIZE), bytes(rgba))
    gdi32.DeleteObject(hbmp)
    gdi32.DeleteDC(hdc)
    user32.ReleaseDC(0, hdc_screen)
    return img


def extract(name: str, directory: bool) -> Image.Image:
    info = SHFILEINFOW()
    flags = SHGFI_ICON | SHGFI_USEFILEATTRIBUTES
    attrs = FILE_ATTRIBUTE_DIRECTORY if directory else FILE_ATTRIBUTE_NORMAL
    ok = shell32.SHGetFileInfoW(
        ctypes.c_wchar_p(name),
        attrs,
        ctypes.byref(info),
        ctypes.sizeof(info),
        flags,
    )
    if not ok or not info.hIcon:
        raise RuntimeError(f"SHGetFileInfo failed for {name!r}")
    try:
        return to_png(info.hIcon)
    finally:
        user32.DestroyIcon(info.hIcon)


def main() -> None:
    out = Path(__file__).resolve().parent / "shell"
    out.mkdir(parents=True, exist_ok=True)
    for kind, (name, is_dir) in KINDS.items():
        img = extract(name, is_dir)
        dest = out / f"{kind}.png"
        img.save(dest)
        print("wrote", dest.name, img.size)


if __name__ == "__main__":
    main()
