"""Work-based extraction progress. ETA is explicitly an estimate, never a timer-driven percent."""
import json
import math
from pathlib import Path
import threading
import time

PREFIX = "@@RRBTD6_PROGRESS "
# Relative first-run work estimates. Completed stage timings replace these locally.
# Measured full run, mostly one worker (2026-11-01); only the stages' relative sizes matter.
SECONDS = {"metadata": 10, "sim_model": 12, "cab_index": 19, "catalogs": 9,
           "container_index": 9, "rounds_maps": 100, "sprites": 15, "mapset": 2,
           "tower_models": 450, "misc_art": 32, "upgrade_art": 22, "ingame_ui": 40,
           "core_menus": 31, "game_data": 21, "extended_towers": 550,
           "tower_indices": 2, "particles": 400, "audio": 25, "bloon_models": 20,
           "frontend": 113, "startup": 40, "home": 100, "social": 95,
           "menu_additions": 40, "profile_menu": 30, "builtin_ui_sprites": 10, "map_mechanics": 60, "finish": 69, "dedupe": 17, "prune_achievements": 1}
LABELS = {"rounds_maps": "Preparing maps", "tower_models": "Preparing tower models",
          "extended_towers": "Preparing heroes and remaining towers", "finish": "Compressing assets",
          "dedupe": "Sharing duplicate files", "prune_achievements": "Removing unused achievement assets",
          "profile_menu": "Preparing the profile page", "builtin_ui_sprites": "Preparing UI sprites",
          "map_mechanics": "Reading map mechanics"}


def duration(seconds):
    if not math.isfinite(seconds) or seconds < 0:
        return "estimating"
    minutes = max(1, math.ceil(seconds / 60))
    return f"{minutes}m" if minutes < 60 else f"{minutes // 60}h {minutes % 60:02}m"


class Progress:
    def __init__(self, stages, history=None, emit=print, clock=time.monotonic):
        self.stages = list(stages)
        self.history = {k: v for k, v in (history or {}).items()
                        if isinstance(v, (int, float)) and math.isfinite(v) and v > 0}
        self.weights = {k: self.history.get(k, SECONDS.get(k, 60)) for k in stages}
        self.emit, self.clock = emit, clock
        self.done, self.current, self.fraction, self.label = set(), None, 0., "Preparing setup"
        self.began = self.stage_began = clock()
        self.observed_weight = self.observed_time = 0.
        self.lock = threading.RLock()
        self.stop_event = threading.Event()
        self.thread = None
        self.success = False

    def line(self):
        with self.lock:
            total = max(sum(self.weights.values()), 1.)
            work = sum(self.weights[k] for k in self.done)
            if self.current:
                work += self.weights[self.current] * self.fraction
            percent = 100. if self.success else min(99.5, 100. * work / total)
            filled = min(28, int(percent * 28 / 100))
            eta = "ETA: estimating..."
            if self.success:
                eta = "Ready to play"
            elif self.history or self.observed_weight >= 60:
                sample_weight, sample_time = self.observed_weight, self.observed_time
                if self.current and self.fraction >= .02:
                    sample_weight += self.weights[self.current] * self.fraction
                    sample_time += max(0., self.clock() - self.stage_began)
                factor = max(.05, sample_time / sample_weight) if sample_weight else 1.
                remaining = max(0., total - work) * factor
                eta = f"ETA: ~{duration(remaining)} (estimate)"
            return f"[{'#' * filled}{'-' * (28 - filled)}] {percent:5.1f}% | {self.label} | {eta}"

    def show(self):
        self.emit(self.line(), flush=True)

    def start(self, name):
        with self.lock:
            self.current, self.fraction = name, 0.
            self.label = LABELS.get(name, name.replace('_', ' ').capitalize())
            self.stage_began = self.clock()
        self.show()

    def update(self, done, total, label="", span=(0., 1.)):
        if total <= 0:
            return
        with self.lock:
            self.fraction = max(self.fraction, min(.995, span[0] + span[1] * max(0., min(1., done / total))))
            if label:
                self.label = str(label).replace('\n', ' ')[:95]

    def child(self, line, span=(0., 1.)):
        if not line.startswith(PREFIX):
            return False
        try:
            data = json.loads(line[len(PREFIX):])
            self.update(float(data['done']), float(data['total']), data.get('label', ''), span)
        except (ValueError, KeyError, TypeError):
            return False
        return True

    def complete(self, name, skipped=False):
        with self.lock:
            if not skipped:
                elapsed = max(.001, self.clock() - self.stage_began)
                self.observed_time += elapsed
                self.observed_weight += self.weights[name]
                self.history[name] = elapsed
            self.done.add(name)
            self.current, self.fraction = None, 0.
        self.show()

    def begin(self):
        self.show()
        def tick():
            while not self.stop_event.wait(5):
                self.show()
        self.thread = threading.Thread(target=tick, daemon=True)
        self.thread.start()

    def finish(self, success):
        self.stop_event.set()
        if self.thread:
            self.thread.join(timeout=2)
        with self.lock:
            self.success = success
            self.label = "Setup complete" if success else "Setup stopped; completed work is saved for retry"
        self.show()


def load_history(path):
    try:
        value = json.loads(Path(path).read_text(encoding="utf-8"))
        return value if isinstance(value, dict) else {}
    except (OSError, ValueError):
        return {}
