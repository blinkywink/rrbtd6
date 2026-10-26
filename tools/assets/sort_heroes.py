"""Order ui/runtime/hero_shop.json like HeroUpgradeDetails.SortHerosByUnlockLevel.

Key per hero (TowerDetailsModelSort.unlockLevel): RankInfo.GetHeroUnlockLevel (first 1-based rank
whose levelUnlockString contains the id, else -1); the newest hero 400; other -1 heroes except
Quincy: visiblePlayerLevel + 600, or towerIndex + 500 while on a hero sale (needs live data, not
applied offline). Sorted with List.Sort's introsort (reference source ArraySortHelper), which is
not stable, so ties land where .NET puts them.
Usage: python -I sort_heroes.py EXTRACTED
"""
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
shop = json.loads((root / 'ui/runtime/hero_shop.json').read_text(encoding='utf8'))
unlocks = json.loads((root / 'ui/runtime/rank_info.json').read_text(encoding='utf8')).get('unlockStrings', [])
details = {}
for line in (root / 'decoded/named/objects.jsonl').open(encoding='utf8'):
    if '"HeroDetailsModel"' in line:
        item = json.loads(line)
        if item['type'] == 'HeroDetailsModel':
            details[item['f']['towerId']] = item['f']


def key(hero):
    d = details.get(hero, {})
    level = next((i + 1 for i, s in enumerate(unlocks) if s and hero in s), -1)
    if d.get('newestHero'):
        return 400
    if level == -1 and hero != 'Quincy':
        return d.get('visiblePlayerLevel', 0) + 600
    return level


def intro_sort(keys, cmp):
    """System.Collections.Generic.ArraySortHelper<T>.IntrospectiveSort (reference source)."""
    def swap_if_greater(a, b):
        if a != b and cmp(keys[a], keys[b]) > 0:
            keys[a], keys[b] = keys[b], keys[a]

    def insertion(lo, hi):
        for i in range(lo, hi):
            j, t = i, keys[i + 1]
            while j >= lo and cmp(t, keys[j]) < 0:
                keys[j + 1] = keys[j]; j -= 1
            keys[j + 1] = t

    def heapsort(lo, hi):
        n = hi - lo + 1
        def down(i, n):
            d = keys[lo + i - 1]
            while i <= n // 2:
                child = 2 * i
                if child < n and cmp(keys[lo + child - 1], keys[lo + child]) < 0:
                    child += 1
                if not cmp(d, keys[lo + child - 1]) < 0:
                    break
                keys[lo + i - 1] = keys[lo + child - 1]; i = child
            keys[lo + i - 1] = d
        for i in range(n // 2, 0, -1):
            down(i, n)
        for i in range(n, 1, -1):
            keys[lo], keys[lo + i - 1] = keys[lo + i - 1], keys[lo]
            down(1, i - 1)

    def partition(lo, hi):
        mid = lo + ((hi - lo) >> 1)
        swap_if_greater(lo, mid); swap_if_greater(lo, hi); swap_if_greater(mid, hi)
        pivot = keys[mid]
        keys[mid], keys[hi - 1] = keys[hi - 1], keys[mid]
        left, right = lo, hi - 1
        while left < right:
            left += 1
            while cmp(keys[left], pivot) < 0: left += 1
            right -= 1
            while cmp(pivot, keys[right]) < 0: right -= 1
            if left >= right: break
            keys[left], keys[right] = keys[right], keys[left]
        keys[left], keys[hi - 1] = keys[hi - 1], keys[left]
        return left

    def sort(lo, hi, depth):
        while hi > lo:
            size = hi - lo + 1
            if size <= 16:
                if size == 1: return
                if size == 2: swap_if_greater(lo, hi); return
                if size == 3:
                    swap_if_greater(lo, hi - 1); swap_if_greater(lo, hi); swap_if_greater(hi - 1, hi); return
                insertion(lo, hi); return
            if depth == 0:
                heapsort(lo, hi); return
            depth -= 1
            p = partition(lo, hi)
            sort(p + 1, hi, depth)
            hi = p - 1

    if len(keys) > 1:
        # 2 * FloorLog2(array length); only reached by the heapsort fallback.
        sort(0, len(keys) - 1, 2 * (max(len(keys), 1).bit_length() - 1))


entries = [(h, key(h['id'])) for h in shop]
intro_sort(entries, lambda a, b: a[1] - b[1])
(root / 'ui/runtime/hero_shop.json').write_text(json.dumps([h for h, _ in entries]), encoding='utf8')
print([f"{h['id']}:{k}" for h, k in entries])
