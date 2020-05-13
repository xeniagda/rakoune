import random
import time

import numpy as np
import matplotlib.pyplot as plt

CHECK_DIST = 3

class Packing:
    def __init__(self, height, width):
        self.height = height
        self.width = width

        # csrs[y, x] = sum(csrs[0:y, 0:x])
        self.content_subrect_sum = np.zeros((height + 1, width + 1))

        self.current_rectangles = {} # {id: (y, x, rheight, rwidth)}, all rectangles are [y .. y + rheight), [x .. x + rwidth)

    def get_subrect_sum(self, y, x, rheight, rwidth):
        if y < 0:
            area_above = -y * rwidth
            return area_above + self.get_subrect_sum(0, x, rheight + y, rwidth)

        if x < 0:
            area_left = -x * rheight
            return area_left + self.get_subrect_sum(y, 0, rheight, rwidth + x)

        if y + rheight > self.height:
            area_below = (y + rheight - self.height) * rwidth
            return area_below + self.get_subrect_sum(y, x, self.height - y, rwidth)

        if x + rwidth > self.width:
            area_right = (x + rwidth - self.width) * rheight
            return area_right + self.get_subrect_sum(y, x, rheight, self.width - x)

        return \
            self.content_subrect_sum[y + rheight, x + rwidth] - \
            self.content_subrect_sum[y, x + rwidth] - \
            self.content_subrect_sum[y + rheight, x] + \
            self.content_subrect_sum[y, x]

    def get_rectangle_score(self, y, x, rheight, rwidth):
        # We want to maximize how much the rectangle "touches" the stuff around it
        if self.get_subrect_sum(y, x, rheight, rwidth) != 0:
            return 0

        above = self.get_subrect_sum(y - CHECK_DIST, x, CHECK_DIST, rwidth)
        below = self.get_subrect_sum(y + rheight, x, CHECK_DIST, rwidth)
        left  = self.get_subrect_sum(y, x - CHECK_DIST, rheight, CHECK_DIST)
        right = self.get_subrect_sum(y, x + rwidth, rheight, CHECK_DIST)

        return above + below + left + right

    def get_adjacent_points(self, rheight, rwidth):
        points = set()
        # Add edge points
        for y in range(self.height - rheight):
            points.add((y, 0))
            points.add((y, self.width - rwidth))

        for x in range(self.width - rwidth):
            points.add((0, x))
            points.add((self.height - rheight, x))

        # Add points adjacent to existing rectangles
        for (ry, rx, rrheight, rrwidth) in self.current_rectangles.values():
            for y in range(ry - rheight, ry + rrheight + rheight):
                points.add((y, rx - rwidth))
                points.add((y, rx + rrwidth))
            for x in range(rx - rwidth, rx + rrwidth + rwidth):
                points.add((ry - rheight, x))
                points.add((ry + rrheight, x))

        good_points = [
            (y, x) for y, x in points
            if y >= 0 and y + rheight <= self.height and x >= 0 and x + rwidth <= self.width
        ]
        return good_points

    def add_best_rectangle(self, rid, rheight, rwidth):
        best_score = 0
        best_pos = (0, 0)
        for y, x in self.get_adjacent_points(rheight, rwidth):
            score_here = self.get_rectangle_score(y, x, rheight, rwidth)
            if score_here > best_score:
                best_score = score_here
                best_pos = (y, x)

        return self.try_add_rect(rid, *best_pos, rheight, rwidth)

    def try_add_rect(self, rid, y, x, rheight, rwidth):
        if self.get_subrect_sum(y, x, rheight, rwidth) != 0:
            return False

        self.current_rectangles[rid] = (y, x, rheight, rwidth)

        xs, ys = np.meshgrid(np.arange(self.width + 1), np.arange(self.height + 1))

        y_dists = np.clip(ys - y, 0, rheight)
        x_dists = np.clip(xs - x, 0, rwidth)
        areas = y_dists * x_dists

        self.content_subrect_sum += areas

        return True

    def render(self):
        result = np.zeros((self.height, self.width))

        for rid, (y, x, rheight, rwidth) in self.current_rectangles.items():
            for py in range(y, y + rheight):
                for px in range(x, x + rwidth):
                    result[py, px] = rid

        return result

FONT_SIZE = 20

p = Packing(FONT_SIZE * 16, FONT_SIZE * 16)

i = 0
total_time = 0
print("start")
while True:
    h = int(FONT_SIZE * (random.gauss(0, 0.2) + 1))
    w = int(FONT_SIZE * (random.gauss(0, 0.4) + 1))
    h = max(4, h)
    w = max(4, w)
    start = time.time()
    did_add = p.add_best_rectangle(i + 1, h, w)
    end = time.time()
    total_time += end - start
    i += 1
    if not did_add:
        break


print(f"Took {total_time:.3} seconds to place {i} tiles ({i / total_time:.3} tiles per seconds)");


plt.subplot(211)
plt.imshow(p.render())
plt.subplot(212)
plt.imshow(p.content_subrect_sum)
plt.show()

