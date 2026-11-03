// The setup progress window: a bar, the current stage, the estimated time left and rotating
// tips, fed from the launcher's output so players don't have to read the MelonLoader terminal.
// Runs its own window thread so BTD6 keeps responding. C# 5, like SetupDialog.cs.
using System;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;
using System.Threading;

namespace RRBTD6Installer
{
    public static partial class SetupDialog
    {
        public sealed class Progress
        {
            static readonly string[] Tips =
            {
                "Copying Dart Monkeys...", "Beating CHIMPS up...", "Adding Herobrine...", "Subscribing to blinkywink...",
                "Inflating bloons...", "Sharpening tacks...", "Teaching the MOAB to sit still...", "Watering the Banana Farm...",
                "Giving the Glue Gunner a bath...", "Herding camo bloons...", "Polishing the Sun God...", "Rusting the code...",
                "Asking the Monkey Village for directions...", "Refilling mortar shells...", "Hiding from the BAD...",
                "Restocking Geraldo's shop...", "Untangling boomerangs...", "Counting every pop...", "Waking up the Ninja Monkey...",
                "Defrosting the Ice Monkey...", "Charging the Super Monkey's laser eyes...", "Bribing the Heli Pilot...",
            };
            const int W = 540, Bar = 18;
            static readonly Regex Line = new Regex(@"^\[[#-]+\]\s+([\d.]+)%\s+\|\s+(.*?)\s+\|\s+ETA:\s*(.*)$");

            readonly object gate = new object();
            readonly ManualResetEvent ready = new ManualResetEvent(false);
            readonly Random random = new Random();
            State s;
            IntPtr hwnd;
            float percent = -1, sweep;
            string stage = "Getting ready", eta = "", tip;
            int tipTicks, tipIndex;
            bool closing;
            WndProc proc;

            public Progress()
            {
                tipIndex = random.Next(Tips.Length);
                tip = Tips[tipIndex];
                var thread = new Thread(Run) { IsBackground = true, Name = "RustTD6 setup progress" };
                thread.SetApartmentState(ApartmentState.STA);
                thread.Start();
                ready.WaitOne(5000);
            }

            /// One line of launcher output: an extraction progress line, or a launcher status line.
            public void Report(string text)
            {
                if (string.IsNullOrWhiteSpace(text)) return;
                Match m = Line.Match(text.Trim());
                lock (gate)
                {
                    if (m.Success)
                    {
                        float p;
                        if (float.TryParse(m.Groups[1].Value, System.Globalization.NumberStyles.Float, System.Globalization.CultureInfo.InvariantCulture, out p))
                            percent = Math.Max(0, Math.Min(100, p));
                        stage = m.Groups[2].Value;
                        string e = m.Groups[3].Value.Replace(" (estimate)", "").Trim();
                        eta = e.StartsWith("~") ? "About " + e.Substring(1) + " left" : e.StartsWith("estimating") ? "Estimating time left..." : "";
                    }
                    else if (percent < 0 && !text.StartsWith(" ") && !text.StartsWith("==") && text.Length < 120 &&
                             !text.StartsWith("RustTD6 setup") && !text.StartsWith("Time remaining") && !text.StartsWith("Setup uses") && !text.StartsWith("Install data"))
                        stage = text.Trim().TrimEnd('.');
                    else return;
                }
                Repaint();
            }

            /// Closes the window (setup finished, failed or the launcher exited).
            public void Close()
            {
                lock (gate) closing = true;
                if (hwnd != IntPtr.Zero) PostMessageW(hwnd, 0x0010 /*WM_CLOSE*/, IntPtr.Zero, IntPtr.Zero);
            }

            void Repaint() { if (hwnd != IntPtr.Zero) InvalidateRect(hwnd, IntPtr.Zero, false); }

            void Run()
            {
                IntPtr token = IntPtr.Zero;
                try
                {
                    try { SetThreadDpiAwarenessContext(new IntPtr(-4)); } catch (EntryPointNotFoundException) { }
                    uint dpi = 96, dpiY;
                    try { if (GetDpiForMonitor(MonitorFromWindow(IntPtr.Zero, 1 /*PRIMARY*/), 0, out dpi, out dpiY) != 0) dpi = 96; }
                    catch (DllNotFoundException) { } catch (EntryPointNotFoundException) { }
                    s = new State { Scale = dpi == 0 ? 1f : dpi / 96f, Title = "Setting up RustTD6", Kind = DialogKind.Warning, Buttons = new Button[0],
                                    Note = "Setup uses a lot of CPU, memory and disk while it runs. It goes faster with more free memory, so close programs you aren't using, then let it finish." };
                    s.TitleFont = Font(s, 23, 700, Display); s.HeadingFont = Font(s, 18, 700, Display);
                    s.BodyFont = Font(s, 15, 400, Reading); s.NoteFont = Font(s, 14, 400, Reading); s.ButtonFont = Font(s, 16, 400, Display);
                    // The note's box fits its wrapped text at this monitor's scale.
                    IntPtr screen = GetDC(IntPtr.Zero), measure = CreateCompatibleDC(screen);
                    Rect text = Measure(measure, s.NoteFont, s.Note, Px(s, Pad + 14), Px(s, 236 + 12), Px(s, W - 2 * Pad - 28));
                    DeleteDC(measure); ReleaseDC(IntPtr.Zero, screen);
                    s.NoteBox = new Rect { Left = Px(s, Pad), Top = Px(s, 236), Right = Px(s, W - Pad), Bottom = text.Bottom + Px(s, 12) };
                    s.Height = s.NoteBox.Bottom + Px(s, Pad);
                    token = StartGdiPlus();
                    proc = Proc;
                    IntPtr instance = GetModuleHandleW(null);
                    var wc = new WNDCLASSEX();
                    wc.cbSize = (uint)Marshal.SizeOf(typeof(WNDCLASSEX));
                    wc.style = 0x20000;
                    wc.lpfnWndProc = Marshal.GetFunctionPointerForDelegate(proc);
                    wc.hInstance = instance;
                    wc.hCursor = LoadCursorW(IntPtr.Zero, new IntPtr(32512));
                    wc.lpszClassName = "RustTD6SetupProgress";
                    RegisterClassExW(ref wc);
                    int w = Px(s, W), h = s.Height;
                    hwnd = CreateWindowExW(0x8 /*TOPMOST*/, "RustTD6SetupProgress", "RustTD6 setup", 0x80000000 /*POPUP*/,
                                           (GetSystemMetrics(0) - w) / 2, (GetSystemMetrics(1) - h) / 2, w, h, IntPtr.Zero, IntPtr.Zero, instance, IntPtr.Zero);
                    if (hwnd == IntPtr.Zero) return;
                    int round = 2;
                    try { DwmSetWindowAttribute(hwnd, 33, ref round, 4); } catch (DllNotFoundException) { }
                    ShowWindow(hwnd, 5);
                    SetTimer(hwnd, new IntPtr(1), 33, IntPtr.Zero);
                    ready.Set();
                    // Close() may have run before the window existed.
                    bool closed; lock (gate) closed = closing;
                    if (closed) PostMessageW(hwnd, 0x0010, IntPtr.Zero, IntPtr.Zero);
                    MSG msg;
                    while (GetMessageW(out msg, IntPtr.Zero, 0, 0) > 0)
                    {
                        TranslateMessage(ref msg);
                        DispatchMessageW(ref msg);
                    }
                }
                catch (Exception) { /* The terminal still shows progress. */ }
                finally
                {
                    ready.Set();
                    if (s != null) Release(s);
                    if (token != IntPtr.Zero) GdiplusShutdown(token);
                }
            }

            IntPtr Proc(IntPtr h, uint msg, IntPtr wParam, IntPtr lParam)
            {
                switch (msg)
                {
                    case 0x0014: return new IntPtr(1);
                    case 0x0113: // WM_TIMER: the moving bar while there is no percentage yet, and the tips.
                        lock (gate)
                        {
                            sweep = (sweep + 0.012f) % 1.3f;
                            if (++tipTicks >= 105) // about 3.5 s
                            {
                                tipTicks = 0;
                                int next = random.Next(Tips.Length - 1);
                                tipIndex = next >= tipIndex ? next + 1 : next;
                                tip = Tips[tipIndex];
                            }
                        }
                        InvalidateRect(h, IntPtr.Zero, false);
                        return IntPtr.Zero;
                    case 0x0084: return new IntPtr(2); // drag anywhere
                    case 0x0010: // WM_CLOSE: hiding the window never stops setup.
                        DestroyWindow(h);
                        return IntPtr.Zero;
                    case 0x0002: // WM_DESTROY: end this thread's loop only.
                        KillTimer(h, new IntPtr(1));
                        hwnd = IntPtr.Zero;
                        PostQuitMessage(0);
                        return IntPtr.Zero;
                    case 0x000F:
                    {
                        PAINTSTRUCT ps;
                        IntPtr dc = BeginPaint(h, out ps);
                        Rect client; GetClientRect(h, out client);
                        IntPtr memory = CreateCompatibleDC(dc), bitmap = CreateCompatibleBitmap(dc, client.Right, client.Bottom);
                        IntPtr old = SelectObject(memory, bitmap);
                        lock (gate) Paint(memory, client.Right, client.Bottom);
                        BitBlt(dc, 0, 0, client.Right, client.Bottom, memory, 0, 0, 0x00CC0020);
                        SelectObject(memory, old); DeleteObject(bitmap); DeleteDC(memory);
                        EndPaint(h, ref ps);
                        return IntPtr.Zero;
                    }
                }
                return DefWindowProcW(h, msg, wParam, lParam);
            }

            void Paint(IntPtr dc, int width, int height)
            {
                IntPtr g;
                GdipCreateFromHDC(dc, out g);
                GdipSetSmoothingMode(g, 4);
                FillRect(g, 0, 0, width, height, Background);
                var top = new PointI { X = 0, Y = 0 }; var bottom = new PointI { X = 0, Y = Px(s, HeaderHeight) };
                IntPtr gradient;
                GdipCreateLineBrushI(ref top, ref bottom, HeaderTop, HeaderBottom, 0, out gradient);
                GdipFillRectangleI(g, gradient, 0, 0, width, Px(s, HeaderHeight));
                GdipDeleteBrush(gradient);
                int x = Px(s, Pad), barY = Px(s, 128), barW = width - 2 * x, barH = Px(s, Bar);
                RoundRect(g, x, barY, barW, barH, barH / 2, 0xFFDDE3EB);
                if (percent >= 0)
                {
                    int fill = (int)(barW * percent / 100f);
                    if (fill > 0) RoundRect(g, x, barY, Math.Max(fill, barH), barH, barH / 2, PrimaryFill);
                }
                else
                {
                    // Indeterminate: a segment sliding along the track.
                    int seg = barW / 4, start = x + (int)((sweep - 0.3f) * barW);
                    int a = Math.Max(x, start), b = Math.Min(x + barW, start + seg);
                    if (b - a > barH / 2) RoundRect(g, a, barY, b - a, barH, barH / 2, PrimaryFill);
                }
                uint noteFill, noteBorder, ink;
                Accent(DialogKind.Warning, out noteFill, out noteBorder, out ink);
                Rect n = s.NoteBox;
                RoundRect(g, n.Left, n.Top, n.Right - n.Left, n.Bottom - n.Top, Px(s, 8), noteBorder);
                RoundRect(g, n.Left + 1, n.Top + 1, n.Right - n.Left - 2, n.Bottom - n.Top - 2, Px(s, 8) - 1, noteFill);
                GdipDeleteGraphics(g);

                SetBkMode(dc, 1);
                Rect r = new Rect { Left = x, Top = 0, Right = width - x, Bottom = Px(s, HeaderHeight) };
                Text(dc, s.TitleFont, s.Title, ref r, 0xFFFFFFFF, 0x20 | 0x4 | 0x800);
                r = new Rect { Left = x, Top = Px(s, 88), Right = width - x, Bottom = Px(s, 120) };
                Text(dc, s.HeadingFont, stage, ref r, HeadingColor, 0x20 | 0x4 | 0x800 | 0x8000 /*END_ELLIPSIS*/);
                r = new Rect { Left = x, Top = barY + barH + Px(s, 6), Right = width - x, Bottom = barY + barH + Px(s, 32) };
                if (percent >= 0) Text(dc, s.BodyFont, ((int)percent) + "%", ref r, BodyColor, 0x20 | 0x4 | 0x800);
                Text(dc, s.BodyFont, eta, ref r, BodyColor, 0x2 /*RIGHT*/ | 0x20 | 0x4 | 0x800);
                r = new Rect { Left = x, Top = Px(s, 186), Right = width - x, Bottom = Px(s, 222) };
                Text(dc, s.ButtonFont, tip, ref r, 0xFF6B7684, 0x1 | 0x20 | 0x4 | 0x800);
                Rect body = new Rect { Left = n.Left + Px(s, 14), Top = n.Top + Px(s, 12), Right = n.Right - Px(s, 14), Bottom = n.Bottom };
                Text(dc, s.NoteFont, s.Note, ref body, ink, 0x10 | 0x800);
            }

            [DllImport("user32.dll")] static extern IntPtr SetTimer(IntPtr hwnd, IntPtr id, uint ms, IntPtr callback);
            [DllImport("user32.dll")] static extern bool KillTimer(IntPtr hwnd, IntPtr id);
            [DllImport("user32.dll")] static extern void PostQuitMessage(int code);
        }
    }
}
