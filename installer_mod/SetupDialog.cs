// The installer's own dialog window: a rounded, borderless card with a blue header, body text,
// an optional highlighted note and large buttons, drawn with GDI+ (anti-aliased shapes) and
// GDI ClearType text. Plain Win32: MelonLoader's .NET runtime has no WinForms/WPF.
// Written in C# 5 so tools can compile it standalone (csc from .NET Framework) for previews.
using System;
using System.Runtime.InteropServices;

namespace RRBTD6Installer
{
    public enum DialogKind { Info, Warning, Error }

    public static partial class SetupDialog
    {
        // Layout in device-independent pixels, scaled by the monitor DPI.
        const int Width = 540, CheckSize = 20, Pad = 28, HeaderHeight = 68, ButtonHeight = 40, ButtonMinWidth = 132, Gap = 12, Radius = 10;

        class Button { public string Text; public int Result; public bool Primary; public Rect Box; }

        class State
        {
            public string Title, Heading, Body, Note, Check;
            public bool Checked;
            public Rect CheckBox;
            public DialogKind Kind;
            public Button[] Buttons;
            public int Cancel, Hover = -1, Pressed = -1, Result, Height;
            public float Scale;
            public bool Done;
            public IntPtr TitleFont, HeadingFont, BodyFont, NoteFont, ButtonFont;
            public Rect HeadingBox, BodyBox, NoteBox;
        }

        static State current;
        static WndProc procedure = Proc;
        static bool registered;

        /// Shows the dialog modally over the active window. buttons: labels, the first is the
        /// primary action (Enter). Returns the clicked button's index, or `cancel` (Esc).
        /// Throws when the window cannot be created; callers fall back to a MessageBox.
        public static int Show(string title, string heading, string body, string note, DialogKind kind, string[] buttons, int cancel)
        {
            bool unused = false;
            return Show(title, heading, body, note, kind, buttons, cancel, null, ref unused);
        }

        /// As Show, with a checkbox labelled `check` (none when null) above the buttons.
        public static int Show(string title, string heading, string body, string note, DialogKind kind, string[] buttons, int cancel, string check, ref bool isChecked)
        {
            IntPtr owner = GetActiveWindow();
            // BTD6 is DPI-unaware: Windows would scale this window as a bitmap (blurry text).
            // A per-monitor aware thread draws it at the monitor's real resolution instead.
            IntPtr previousAwareness = IntPtr.Zero;
            try { previousAwareness = SetThreadDpiAwarenessContext(new IntPtr(-4) /*PER_MONITOR_AWARE_V2*/); } catch (EntryPointNotFoundException) { }
            uint dpi = 96, dpiY;
            try
            {
                IntPtr monitor = MonitorFromWindow(owner, 2 /*MONITOR_DEFAULTTONEAREST*/);
                if (GetDpiForMonitor(monitor, 0 /*MDT_EFFECTIVE_DPI*/, out dpi, out dpiY) != 0) dpi = 96;
            }
            catch (DllNotFoundException) { }
            catch (EntryPointNotFoundException) { }
            State s = Prepare(title, heading, body, note, kind, buttons, cancel, dpi == 0 ? 1f : dpi / 96f, check, isChecked);
            IntPtr token = StartGdiPlus();
            IntPtr hwnd = IntPtr.Zero;
            try
            {
                IntPtr instance = GetModuleHandleW(null);
                if (!registered)
                {
                    var wc = new WNDCLASSEX();
                    wc.cbSize = (uint)Marshal.SizeOf(typeof(WNDCLASSEX));
                    wc.style = 0x20000 /*CS_DROPSHADOW*/;
                    wc.lpfnWndProc = Marshal.GetFunctionPointerForDelegate(procedure);
                    wc.hInstance = instance;
                    wc.hCursor = LoadCursorW(IntPtr.Zero, new IntPtr(32512));
                    wc.lpszClassName = "RustTD6SetupDialog";
                    if (RegisterClassExW(ref wc) == 0) throw new InvalidOperationException("RegisterClassEx failed");
                    registered = true;
                }
                int w = Px(s, Width), h = s.Height;
                Rect area;
                if (owner == IntPtr.Zero || !GetWindowRect(owner, out area))
                    area = new Rect { Left = 0, Top = 0, Right = GetSystemMetrics(0), Bottom = GetSystemMetrics(1) };
                int x = area.Left + (area.Right - area.Left - w) / 2, y = area.Top + (area.Bottom - area.Top - h) / 2;
                current = s;
                hwnd = CreateWindowExW(0x8 /*WS_EX_TOPMOST*/, "RustTD6SetupDialog", title, 0x80000000 /*WS_POPUP*/,
                                       x, y, w, h, owner, IntPtr.Zero, instance, IntPtr.Zero);
                if (hwnd == IntPtr.Zero) throw new InvalidOperationException("CreateWindowEx failed");
                int round = 2; // DWMWCP_ROUND: Windows 11 rounds the corners and keeps the shadow.
                try { DwmSetWindowAttribute(hwnd, 33, ref round, 4); } catch (DllNotFoundException) { }
                if (owner != IntPtr.Zero) EnableWindow(owner, false);
                ShowWindow(hwnd, 5);
                SetForegroundWindow(hwnd);
                SetFocus(hwnd);
                MSG msg;
                while (!s.Done && GetMessageW(out msg, IntPtr.Zero, 0, 0) > 0)
                {
                    TranslateMessage(ref msg);
                    DispatchMessageW(ref msg);
                }
                isChecked = s.Checked;
                return s.Result;
            }
            finally
            {
                if (owner != IntPtr.Zero) { EnableWindow(owner, true); SetForegroundWindow(owner); }
                if (previousAwareness != IntPtr.Zero) SetThreadDpiAwarenessContext(previousAwareness);
                if (hwnd != IntPtr.Zero && IsWindow(hwnd)) DestroyWindow(hwnd);
                Release(s);
                current = null;
                if (token != IntPtr.Zero) GdiplusShutdown(token);
            }
        }

        /// Draws the dialog into a 32-bit top-down BGRA buffer (previews without a window).
        public static byte[] Render(string title, string heading, string body, string note, DialogKind kind, string[] buttons, float scale, string check, bool isChecked, out int width, out int height)
        {
            State s = Prepare(title, heading, body, note, kind, buttons, buttons.Length - 1, scale, check, isChecked);
            IntPtr token = StartGdiPlus();
            try
            {
                width = Px(s, Width); height = s.Height;
                var info = new BITMAPINFOHEADER { biSize = 40, biWidth = width, biHeight = -height, biPlanes = 1, biBitCount = 32 };
                IntPtr bits, screen = GetDC(IntPtr.Zero), dc = CreateCompatibleDC(screen);
                IntPtr bitmap = CreateDIBSection(dc, ref info, 0, out bits, IntPtr.Zero, 0);
                IntPtr old = SelectObject(dc, bitmap);
                Paint(s, dc, width, height);
                var pixels = new byte[width * height * 4];
                Marshal.Copy(bits, pixels, 0, pixels.Length);
                SelectObject(dc, old); DeleteObject(bitmap); DeleteDC(dc); ReleaseDC(IntPtr.Zero, screen);
                return pixels;
            }
            finally { Release(s); GdiplusShutdown(token); }
        }

        static int Px(State s, int dip) { return (int)Math.Round(dip * s.Scale); }

        static State Prepare(string title, string heading, string body, string note, DialogKind kind, string[] labels, int cancel, float scale, string check, bool isChecked)
        {
            var s = new State { Title = title, Heading = heading, Body = body, Note = note, Kind = kind, Cancel = cancel, Scale = scale, Check = check, Checked = isChecked };
            s.TitleFont = Font(s, 23, 700, Display); s.HeadingFont = Font(s, 18, 700, Display); s.BodyFont = Font(s, 15, 400, Reading);
            s.NoteFont = Font(s, 14, 400, Reading); s.ButtonFont = Font(s, 15, 700, Display);
            s.Buttons = new Button[labels.Length];
            for (int i = 0; i < labels.Length; i++) s.Buttons[i] = new Button { Text = labels[i], Result = i, Primary = i == 0 };
            // Measure the wrapped text to size the window.
            IntPtr screen = GetDC(IntPtr.Zero), dc = CreateCompatibleDC(screen);
            int inner = Px(s, Width - 2 * Pad), y = Px(s, HeaderHeight + 24);
            s.HeadingBox = Measure(dc, s.HeadingFont, heading, Px(s, Pad), y, inner);
            y = s.HeadingBox.Bottom + Px(s, 10);
            s.BodyBox = Measure(dc, s.BodyFont, body, Px(s, Pad), y, inner);
            y = s.BodyBox.Bottom;
            if (!string.IsNullOrEmpty(note))
            {
                int notePad = Px(s, 14);
                Rect text = Measure(dc, s.NoteFont, note, Px(s, Pad) + notePad, y + Px(s, 18) + notePad, inner - 2 * notePad);
                s.NoteBox = new Rect { Left = Px(s, Pad), Top = y + Px(s, 18), Right = Px(s, Width - Pad), Bottom = text.Bottom + notePad };
                y = s.NoteBox.Bottom;
            }
            // Buttons right-aligned, primary rightmost, as Windows orders them.
            // An optional checkbox gets its own row under the text; buttons go below it.
            int checkRow = string.IsNullOrEmpty(check) ? 0 : Px(s, 22 + CheckSize);
            int right = Px(s, Width - Pad), top = y + Px(s, 26) + checkRow;
            IntPtr old = SelectObject(dc, s.ButtonFont);
            for (int i = 0; i < s.Buttons.Length; i++)
            {
                Size size;
                GetTextExtentPoint32W(dc, s.Buttons[i].Text, s.Buttons[i].Text.Length, out size);
                int w = Math.Max(Px(s, ButtonMinWidth), size.cx + Px(s, 40));
                s.Buttons[i].Box = new Rect { Left = right - w, Top = top, Right = right, Bottom = top + Px(s, ButtonHeight) };
                right -= w + Px(s, Gap);
            }
            SelectObject(dc, old);
            if (!string.IsNullOrEmpty(check))
            {
                // The checkbox and its label, left-aligned on its row.
                Size size;
                old = SelectObject(dc, s.BodyFont);
                GetTextExtentPoint32W(dc, check, check.Length, out size);
                SelectObject(dc, old);
                int box = Px(s, CheckSize), cy = y + Px(s, 22) + box / 2;
                s.CheckBox = new Rect { Left = Px(s, Pad), Top = cy - box / 2, Right = Px(s, Pad) + box + Px(s, 8) + size.cx, Bottom = cy + box / 2 };
            }
            DeleteDC(dc); ReleaseDC(IntPtr.Zero, screen);
            s.Height = top + Px(s, ButtonHeight) + Px(s, Pad);
            return s;
        }

        static Rect Measure(IntPtr dc, IntPtr font, string text, int x, int y, int width)
        {
            var r = new Rect { Left = x, Top = y, Right = x + width, Bottom = y };
            if (string.IsNullOrEmpty(text)) return r;
            IntPtr old = SelectObject(dc, font);
            DrawTextW(dc, text, -1, ref r, 0x10 | 0x400 | 0x800 /*WORDBREAK|CALCRECT|NOPREFIX*/);
            SelectObject(dc, old);
            r.Right = x + width;
            return r;
        }

        // As the website: Comic Sans for titles, headings and buttons; its body font (Inter,
        // else the system UI font) for the smaller, longer text.
        const string Display = "Comic Sans MS", Reading = "Segoe UI";

        static IntPtr Font(State s, int size, int weight, string face)
        {
            return CreateFontW(-Px(s, size), 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5 /*CLEARTYPE_QUALITY*/, 0, face);
        }

        static void Release(State s)
        {
            foreach (IntPtr f in new[] { s.TitleFont, s.HeadingFont, s.BodyFont, s.NoteFont, s.ButtonFont })
                if (f != IntPtr.Zero) DeleteObject(f);
        }

        static IntPtr StartGdiPlus()
        {
            var input = new GdiplusStartupInput { GdiplusVersion = 1 };
            IntPtr token;
            return GdiplusStartup(out token, ref input, IntPtr.Zero) == 0 ? token : IntPtr.Zero;
        }

        // Colors as 0xAARRGGBB.
        const uint Background = 0xFFF6F8FB, HeaderTop = 0xFF3A8DF0, HeaderBottom = 0xFF1C64D0, HeadingColor = 0xFF1B2533,
                   BodyColor = 0xFF46505E, PrimaryFill = 0xFF33A852, PrimaryHover = 0xFF2C9447, PrimaryDown = 0xFF237A3A,
                   SecondaryFill = 0xFFE4E8EE, SecondaryHover = 0xFFD7DCE4, SecondaryDown = 0xFFC9CFD9, SecondaryText = 0xFF2B3440;

        static void Accent(DialogKind kind, out uint fill, out uint border, out uint text)
        {
            if (kind == DialogKind.Error) { fill = 0xFFFDECEC; border = 0xFFF0A3A3; text = 0xFF8A1C1C; }
            else if (kind == DialogKind.Warning) { fill = 0xFFFFF6E0; border = 0xFFF2C46B; text = 0xFF6E4600; }
            else { fill = 0xFFE8F1FD; border = 0xFFA9C9F3; text = 0xFF1C4A86; }
        }

        static void Paint(State s, IntPtr dc, int width, int height)
        {
            IntPtr g;
            GdipCreateFromHDC(dc, out g);
            GdipSetSmoothingMode(g, 4 /*AntiAlias*/);
            FillRect(g, 0, 0, width, height, Background);
            var top = new PointI { X = 0, Y = 0 }; var bottom = new PointI { X = 0, Y = Px(s, HeaderHeight) };
            IntPtr gradient;
            GdipCreateLineBrushI(ref top, ref bottom, HeaderTop, HeaderBottom, 0, out gradient);
            GdipFillRectangleI(g, gradient, 0, 0, width, Px(s, HeaderHeight));
            GdipDeleteBrush(gradient);
            if (!string.IsNullOrEmpty(s.Note))
            {
                uint fill, border, ink;
                Accent(s.Kind, out fill, out border, out ink);
                Rect n = s.NoteBox;
                RoundRect(g, n.Left, n.Top, n.Right - n.Left, n.Bottom - n.Top, Px(s, 8), border);
                RoundRect(g, n.Left + 1, n.Top + 1, n.Right - n.Left - 2, n.Bottom - n.Top - 2, Px(s, 8) - 1, fill);
            }
            for (int i = 0; i < s.Buttons.Length; i++)
            {
                Button b = s.Buttons[i];
                bool hover = s.Hover == i, down = hover && s.Pressed == i;
                uint fill = b.Primary ? (down ? PrimaryDown : hover ? PrimaryHover : PrimaryFill)
                                      : (down ? SecondaryDown : hover ? SecondaryHover : SecondaryFill);
                RoundRect(g, b.Box.Left, b.Box.Top, b.Box.Right - b.Box.Left, b.Box.Bottom - b.Box.Top, Px(s, Radius - 2), fill);
            }
            if (!string.IsNullOrEmpty(s.Check))
            {
                int box = Px(s, CheckSize), x = s.CheckBox.Left, y = s.CheckBox.Top;
                bool hover = s.Hover == CheckHit;
                RoundRect(g, x, y, box, box, Px(s, 5), s.Checked ? PrimaryFill : hover ? 0xFF8E99A8 : 0xFFAAB3C0);
                if (!s.Checked) RoundRect(g, x + Px(s, 2), y + Px(s, 2), box - Px(s, 4), box - Px(s, 4), Px(s, 3), 0xFFFFFFFF);
                else
                {
                    IntPtr pen;
                    GdipCreatePen1(0xFFFFFFFF, 2.2f * s.Scale, 2 /*UnitPixel*/, out pen);
                    GdipSetPenStartCap(pen, 2); GdipSetPenEndCap(pen, 2); GdipSetPenLineJoin(pen, 2); // round
                    float u = box / 20f;
                    var tick = new[] { new PointF { X = x + 5 * u, Y = y + 10.5f * u }, new PointF { X = x + 8.5f * u, Y = y + 14 * u }, new PointF { X = x + 15 * u, Y = y + 6.5f * u } };
                    GdipDrawLines(g, pen, tick, 3);
                    GdipDeletePen(pen);
                }
            }
            GdipDeleteGraphics(g);

            SetBkMode(dc, 1 /*TRANSPARENT*/);
            Rect title = new Rect { Left = Px(s, Pad), Top = 0, Right = width - Px(s, Pad), Bottom = Px(s, HeaderHeight) };
            Text(dc, s.TitleFont, s.Title, ref title, 0xFFFFFFFF, 0x20 | 0x4 | 0x800 /*SINGLELINE|VCENTER|NOPREFIX*/);
            Rect r = s.HeadingBox; Text(dc, s.HeadingFont, s.Heading, ref r, HeadingColor, 0x10 | 0x800);
            r = s.BodyBox; Text(dc, s.BodyFont, s.Body, ref r, BodyColor, 0x10 | 0x800);
            if (!string.IsNullOrEmpty(s.Note))
            {
                uint fill, border, ink;
                Accent(s.Kind, out fill, out border, out ink);
                Rect n = s.NoteBox;
                Rect body = new Rect { Left = n.Left + Px(s, 14), Top = n.Top + Px(s, 14), Right = n.Right - Px(s, 14), Bottom = n.Bottom };
                Text(dc, s.NoteFont, s.Note, ref body, ink, 0x10 | 0x800);
            }
            if (!string.IsNullOrEmpty(s.Check))
            {
                Rect label = s.CheckBox; label.Left += Px(s, CheckSize) + Px(s, 8);
                label.Top -= Px(s, 6); label.Bottom += Px(s, 6);
                Text(dc, s.BodyFont, s.Check, ref label, HeadingColor, 0x20 | 0x4 | 0x800);
            }
            foreach (Button b in s.Buttons)
            {
                Rect box = b.Box;
                Text(dc, s.ButtonFont, b.Text, ref box, b.Primary ? 0xFFFFFFFF : SecondaryText, 0x1 | 0x20 | 0x4 | 0x800);
            }
        }

        static void Text(IntPtr dc, IntPtr font, string text, ref Rect r, uint argb, uint format)
        {
            if (string.IsNullOrEmpty(text)) return;
            IntPtr old = SelectObject(dc, font);
            SetTextColor(dc, ((argb & 0xFF) << 16) | (argb & 0xFF00) | ((argb >> 16) & 0xFF));
            DrawTextW(dc, text, -1, ref r, format);
            SelectObject(dc, old);
        }

        static void FillRect(IntPtr g, int x, int y, int w, int h, uint argb)
        {
            IntPtr brush; GdipCreateSolidFill(argb, out brush);
            GdipFillRectangleI(g, brush, x, y, w, h);
            GdipDeleteBrush(brush);
        }

        static void RoundRect(IntPtr g, int x, int y, int w, int h, int radius, uint argb)
        {
            IntPtr path, brush;
            float d = radius * 2f;
            GdipCreatePath(0, out path);
            GdipAddPathArc(path, x, y, d, d, 180, 90);
            GdipAddPathArc(path, x + w - d, y, d, d, 270, 90);
            GdipAddPathArc(path, x + w - d, y + h - d, d, d, 0, 90);
            GdipAddPathArc(path, x, y + h - d, d, d, 90, 90);
            GdipClosePathFigure(path);
            GdipCreateSolidFill(argb, out brush);
            GdipFillPath(g, brush, path);
            GdipDeleteBrush(brush);
            GdipDeletePath(path);
        }

        const int CheckHit = 1000;

        static int HitButton(State s, int x, int y)
        {
            if (!string.IsNullOrEmpty(s.Check) && x >= s.CheckBox.Left && x < s.CheckBox.Right && y >= s.CheckBox.Top - 4 && y < s.CheckBox.Bottom + 4)
                return CheckHit;
            for (int i = 0; i < s.Buttons.Length; i++)
            {
                Rect b = s.Buttons[i].Box;
                if (x >= b.Left && x < b.Right && y >= b.Top && y < b.Bottom) return i;
            }
            return -1;
        }

        static void Finish(IntPtr hwnd, State s, int result)
        {
            s.Result = result;
            s.Done = true;
            // Wake GetMessage; never PostQuitMessage, which would also end the game's own loop.
            PostMessageW(hwnd, 0, IntPtr.Zero, IntPtr.Zero);
        }

        static IntPtr Proc(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam)
        {
            State s = current;
            if (s == null) return DefWindowProcW(hwnd, msg, wParam, lParam);
            int lx = (short)((long)lParam & 0xFFFF), ly = (short)(((long)lParam >> 16) & 0xFFFF);
            switch (msg)
            {
                case 0x0014: return new IntPtr(1); // WM_ERASEBKGND: painted in WM_PAINT.
                case 0x000F: // WM_PAINT, double-buffered.
                {
                    PAINTSTRUCT ps;
                    IntPtr dc = BeginPaint(hwnd, out ps);
                    Rect client; GetClientRect(hwnd, out client);
                    IntPtr memory = CreateCompatibleDC(dc), bitmap = CreateCompatibleBitmap(dc, client.Right, client.Bottom);
                    IntPtr old = SelectObject(memory, bitmap);
                    Paint(s, memory, client.Right, client.Bottom);
                    BitBlt(dc, 0, 0, client.Right, client.Bottom, memory, 0, 0, 0x00CC0020 /*SRCCOPY*/);
                    SelectObject(memory, old); DeleteObject(bitmap); DeleteDC(memory);
                    EndPaint(hwnd, ref ps);
                    return IntPtr.Zero;
                }
                case 0x0084: // WM_NCHITTEST: drag the window by its header.
                {
                    Point p = new Point { X = lx, Y = ly };
                    ScreenToClient(hwnd, ref p);
                    if (p.Y < Px(s, HeaderHeight)) return new IntPtr(2 /*HTCAPTION*/);
                    break;
                }
                case 0x0200: // WM_MOUSEMOVE
                {
                    int hover = HitButton(s, lx, ly);
                    if (hover != s.Hover)
                    {
                        s.Hover = hover;
                        SetCursor(LoadCursorW(IntPtr.Zero, new IntPtr(hover >= 0 ? 32649 /*IDC_HAND*/ : 32512)));
                        InvalidateRect(hwnd, IntPtr.Zero, false);
                        var track = new TRACKMOUSEEVENT { cbSize = (uint)Marshal.SizeOf(typeof(TRACKMOUSEEVENT)), dwFlags = 2 /*TME_LEAVE*/, hwndTrack = hwnd };
                        TrackMouseEvent(ref track);
                    }
                    return IntPtr.Zero;
                }
                case 0x0020: // WM_SETCURSOR
                    if (s.Hover >= 0) { SetCursor(LoadCursorW(IntPtr.Zero, new IntPtr(32649))); return new IntPtr(1); }
                    break;
                case 0x02A3: // WM_MOUSELEAVE
                    s.Hover = -1; s.Pressed = -1; InvalidateRect(hwnd, IntPtr.Zero, false);
                    return IntPtr.Zero;
                case 0x0201: // WM_LBUTTONDOWN
                    s.Pressed = HitButton(s, lx, ly);
                    if (s.Pressed >= 0) { SetCapture(hwnd); InvalidateRect(hwnd, IntPtr.Zero, false); }
                    return IntPtr.Zero;
                case 0x0202: // WM_LBUTTONUP
                {
                    int pressed = s.Pressed;
                    s.Pressed = -1;
                    ReleaseCapture();
                    InvalidateRect(hwnd, IntPtr.Zero, false);
                    if (pressed == CheckHit && HitButton(s, lx, ly) == CheckHit) { s.Checked = !s.Checked; InvalidateRect(hwnd, IntPtr.Zero, false); }
                    else if (pressed >= 0 && pressed == HitButton(s, lx, ly)) Finish(hwnd, s, s.Buttons[pressed].Result);
                    return IntPtr.Zero;
                }
                case 0x0100: // WM_KEYDOWN
                    if ((int)wParam == 0x0D) Finish(hwnd, s, 0);
                    else if ((int)wParam == 0x1B) Finish(hwnd, s, s.Cancel);
                    else if ((int)wParam == 0x20 && !string.IsNullOrEmpty(s.Check)) { s.Checked = !s.Checked; InvalidateRect(hwnd, IntPtr.Zero, false); }
                    return IntPtr.Zero;
                case 0x0010: // WM_CLOSE (Alt+F4)
                    Finish(hwnd, s, s.Cancel);
                    return IntPtr.Zero;
            }
            return DefWindowProcW(hwnd, msg, wParam, lParam);
        }

        delegate IntPtr WndProc(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);

        [StructLayout(LayoutKind.Sequential)] struct Rect { public int Left, Top, Right, Bottom; }
        [StructLayout(LayoutKind.Sequential)] struct Point { public int X, Y; }
        [StructLayout(LayoutKind.Sequential)] struct PointI { public int X, Y; }
        [StructLayout(LayoutKind.Sequential)] struct PointF { public float X, Y; }
        [StructLayout(LayoutKind.Sequential)] struct Size { public int cx, cy; }
        [StructLayout(LayoutKind.Sequential)] struct MSG { public IntPtr hwnd; public uint message; public IntPtr wParam, lParam; public uint time; public Point pt; }
        [StructLayout(LayoutKind.Sequential)] struct PAINTSTRUCT { public IntPtr hdc; public int fErase; public Rect rcPaint; public int fRestore, fIncUpdate; [MarshalAs(UnmanagedType.ByValArray, SizeConst = 32)] public byte[] rgbReserved; }
        [StructLayout(LayoutKind.Sequential)] struct TRACKMOUSEEVENT { public uint cbSize, dwFlags; public IntPtr hwndTrack; public uint dwHoverTime; }
        [StructLayout(LayoutKind.Sequential)] struct GdiplusStartupInput { public uint GdiplusVersion; public IntPtr DebugEventCallback; public int SuppressBackgroundThread, SuppressExternalCodecs; }
        [StructLayout(LayoutKind.Sequential)] struct BITMAPINFOHEADER { public uint biSize; public int biWidth, biHeight; public ushort biPlanes, biBitCount; public uint biCompression, biSizeImage; public int biXPelsPerMeter, biYPelsPerMeter; public uint biClrUsed, biClrImportant; }
        [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
        struct WNDCLASSEX
        {
            public uint cbSize, style; public IntPtr lpfnWndProc; public int cbClsExtra, cbWndExtra;
            public IntPtr hInstance, hIcon, hCursor, hbrBackground; public string lpszMenuName, lpszClassName; public IntPtr hIconSm;
        }

        [DllImport("user32.dll")] static extern IntPtr GetActiveWindow();
        [DllImport("user32.dll")] static extern IntPtr SetThreadDpiAwarenessContext(IntPtr context);
        [DllImport("user32.dll")] static extern IntPtr MonitorFromWindow(IntPtr hwnd, uint flags);
        [DllImport("shcore.dll")] static extern int GetDpiForMonitor(IntPtr monitor, int type, out uint x, out uint y);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern ushort RegisterClassExW(ref WNDCLASSEX wc);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern IntPtr CreateWindowExW(uint exStyle, string cls, string name, uint style, int x, int y, int w, int h, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr param);
        [DllImport("user32.dll")] static extern IntPtr DefWindowProcW(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll")] static extern int GetMessageW(out MSG msg, IntPtr hwnd, uint min, uint max);
        [DllImport("user32.dll")] static extern bool TranslateMessage(ref MSG msg);
        [DllImport("user32.dll")] static extern IntPtr DispatchMessageW(ref MSG msg);
        [DllImport("user32.dll")] static extern bool PostMessageW(IntPtr hwnd, uint msg, IntPtr wParam, IntPtr lParam);
        [DllImport("user32.dll")] static extern bool ShowWindow(IntPtr hwnd, int cmd);
        [DllImport("user32.dll")] static extern bool DestroyWindow(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool IsWindow(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool EnableWindow(IntPtr hwnd, bool enable);
        [DllImport("user32.dll")] static extern bool SetForegroundWindow(IntPtr hwnd);
        [DllImport("user32.dll")] static extern IntPtr SetFocus(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool GetWindowRect(IntPtr hwnd, out Rect rect);
        [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr hwnd, out Rect rect);
        [DllImport("user32.dll")] static extern bool ScreenToClient(IntPtr hwnd, ref Point p);
        [DllImport("user32.dll")] static extern int GetSystemMetrics(int index);
        [DllImport("user32.dll")] static extern bool InvalidateRect(IntPtr hwnd, IntPtr rect, bool erase);
        [DllImport("user32.dll")] static extern IntPtr BeginPaint(IntPtr hwnd, out PAINTSTRUCT ps);
        [DllImport("user32.dll")] static extern bool EndPaint(IntPtr hwnd, ref PAINTSTRUCT ps);
        [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr hwnd);
        [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr hwnd, IntPtr dc);
        [DllImport("user32.dll")] static extern IntPtr SetCapture(IntPtr hwnd);
        [DllImport("user32.dll")] static extern bool ReleaseCapture();
        [DllImport("user32.dll")] static extern IntPtr SetCursor(IntPtr cursor);
        [DllImport("user32.dll")] static extern IntPtr LoadCursorW(IntPtr instance, IntPtr name);
        [DllImport("user32.dll")] static extern bool TrackMouseEvent(ref TRACKMOUSEEVENT e);
        [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int DrawTextW(IntPtr dc, string text, int count, ref Rect rect, uint format);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)] static extern IntPtr GetModuleHandleW(string name);
        [DllImport("dwmapi.dll")] static extern int DwmSetWindowAttribute(IntPtr hwnd, int attribute, ref int value, int size);
        [DllImport("gdi32.dll", CharSet = CharSet.Unicode)] static extern IntPtr CreateFontW(int h, int w, int esc, int orient, int weight, uint italic, uint underline, uint strike, uint charset, uint outPrecision, uint clipPrecision, uint quality, uint pitch, string face);
        [DllImport("gdi32.dll", CharSet = CharSet.Unicode)] static extern bool GetTextExtentPoint32W(IntPtr dc, string text, int count, out Size size);
        [DllImport("gdi32.dll")] static extern IntPtr SelectObject(IntPtr dc, IntPtr obj);
        [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr obj);
        [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleDC(IntPtr dc);
        [DllImport("gdi32.dll")] static extern IntPtr CreateCompatibleBitmap(IntPtr dc, int w, int h);
        [DllImport("gdi32.dll")] static extern IntPtr CreateDIBSection(IntPtr dc, ref BITMAPINFOHEADER info, uint usage, out IntPtr bits, IntPtr section, uint offset);
        [DllImport("gdi32.dll")] static extern bool DeleteDC(IntPtr dc);
        [DllImport("gdi32.dll")] static extern bool BitBlt(IntPtr dc, int x, int y, int w, int h, IntPtr src, int sx, int sy, uint rop);
        [DllImport("gdi32.dll")] static extern int SetBkMode(IntPtr dc, int mode);
        [DllImport("gdi32.dll")] static extern uint SetTextColor(IntPtr dc, uint color);
        [DllImport("gdiplus.dll")] static extern int GdiplusStartup(out IntPtr token, ref GdiplusStartupInput input, IntPtr output);
        [DllImport("gdiplus.dll")] static extern void GdiplusShutdown(IntPtr token);
        [DllImport("gdiplus.dll")] static extern int GdipCreateFromHDC(IntPtr dc, out IntPtr graphics);
        [DllImport("gdiplus.dll")] static extern int GdipDeleteGraphics(IntPtr graphics);
        [DllImport("gdiplus.dll")] static extern int GdipSetSmoothingMode(IntPtr graphics, int mode);
        [DllImport("gdiplus.dll")] static extern int GdipCreateSolidFill(uint argb, out IntPtr brush);
        [DllImport("gdiplus.dll")] static extern int GdipCreateLineBrushI(ref PointI a, ref PointI b, uint argbA, uint argbB, int wrap, out IntPtr brush);
        [DllImport("gdiplus.dll")] static extern int GdipDeleteBrush(IntPtr brush);
        [DllImport("gdiplus.dll")] static extern int GdipFillRectangleI(IntPtr graphics, IntPtr brush, int x, int y, int w, int h);
        [DllImport("gdiplus.dll")] static extern int GdipCreatePath(int fillMode, out IntPtr path);
        [DllImport("gdiplus.dll")] static extern int GdipDeletePath(IntPtr path);
        [DllImport("gdiplus.dll")] static extern int GdipAddPathArc(IntPtr path, float x, float y, float w, float h, float start, float sweep);
        [DllImport("gdiplus.dll")] static extern int GdipClosePathFigure(IntPtr path);
        [DllImport("gdiplus.dll")] static extern int GdipFillPath(IntPtr graphics, IntPtr brush, IntPtr path);
        [DllImport("gdiplus.dll")] static extern int GdipCreatePen1(uint argb, float width, int unit, out IntPtr pen);
        [DllImport("gdiplus.dll")] static extern int GdipDeletePen(IntPtr pen);
        [DllImport("gdiplus.dll")] static extern int GdipSetPenStartCap(IntPtr pen, int cap);
        [DllImport("gdiplus.dll")] static extern int GdipSetPenEndCap(IntPtr pen, int cap);
        [DllImport("gdiplus.dll")] static extern int GdipSetPenLineJoin(IntPtr pen, int join);
        [DllImport("gdiplus.dll")] static extern int GdipDrawLines(IntPtr graphics, IntPtr pen, PointF[] points, int count);
    }
}
