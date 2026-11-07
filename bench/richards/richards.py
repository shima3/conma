"""richards: Martin Richards' simulation of the task dispatcher of an
operating system (the richards benchmark of pyperformance), with tasks as
objects whose kind gives the function that runs them.

    python3 richards.py N

Runs the simulation N times and prints the hold count and the packet count
of each run (9297 and 23246). richards.se does the same operations in the
same order, so both print the same lines."""

import sys

I_IDLE = 1
I_WORK = 2
I_HANDLERA = 3
I_HANDLERB = 4
I_DEVA = 5
I_DEVB = 6

K_DEV = 1000
K_WORK = 1001

BUFSIZE = 4


class Packet:
    def __init__(self, link, ident, kind):
        self.link = link
        self.ident = ident
        self.kind = kind
        self.datum = 0
        self.data = [0] * BUFSIZE

    def append_to(self, lst):
        """lst with this packet at its end."""
        self.link = None
        if lst is None:
            return self
        p = lst
        while p.link is not None:
            p = p.link
        p.link = self
        return lst


class DeviceRec:
    def __init__(self):
        self.pending = None


class IdleRec:
    def __init__(self):
        self.control = 1
        self.count = 10000


class HandlerRec:
    def __init__(self):
        self.work_in = None
        self.device_in = None


class WorkerRec:
    def __init__(self):
        self.destination = I_HANDLERA
        self.count = 0


class Area:
    def __init__(self):
        self.tasks = [None] * 10
        self.task_list = None
        self.hold_count = 0
        self.qpkt_count = 0


area = Area()


class Task:
    def __init__(self, ident, priority, input, pending, waiting, holding, rec):
        self.link = area.task_list
        self.ident = ident
        self.priority = priority
        self.input = input
        self.pending = pending
        self.waiting = waiting
        self.holding = holding
        self.rec = rec
        area.task_list = self
        area.tasks[ident] = self

    def holding_or_waiting(self):
        return self.holding or (not self.pending and self.waiting)

    def waiting_with_packet(self):
        return self.pending and self.waiting and not self.holding

    def add_packet(self, p, old):
        if self.input is None:
            self.input = p
            self.pending = True
            if self.priority > old.priority:
                return self
        else:
            self.input = p.append_to(self.input)
        return old

    def run(self):
        if self.waiting_with_packet():
            msg = self.input
            self.input = msg.link
            if self.input is None:
                self.pending = False
                self.waiting = False
                self.holding = False
            else:
                self.pending = True
                self.waiting = False
                self.holding = False
        else:
            msg = None
        return self.fn(msg, self.rec)

    def wait(self):
        self.waiting = True
        return self

    def hold(self):
        area.hold_count += 1
        self.holding = True
        return self.link

    def release(self, ident):
        t = find_task(ident)
        t.holding = False
        if t.priority > self.priority:
            return t
        return self

    def qpkt(self, pkt):
        t = find_task(pkt.ident)
        area.qpkt_count += 1
        pkt.link = None
        pkt.ident = self.ident
        return t.add_packet(pkt, self)


def find_task(ident):
    t = area.tasks[ident]
    if t is None:
        raise Exception("no task %d" % ident)
    return t


class DeviceTask(Task):
    def fn(self, pkt, d):
        if pkt is None:
            pkt = d.pending
            if pkt is None:
                return self.wait()
            d.pending = None
            return self.qpkt(pkt)
        d.pending = pkt
        return self.hold()


class HandlerTask(Task):
    def fn(self, pkt, h):
        if pkt is not None:
            if pkt.kind == K_WORK:
                h.work_in = pkt.append_to(h.work_in)
            else:
                h.device_in = pkt.append_to(h.device_in)
        work = h.work_in
        if work is None:
            return self.wait()
        count = work.datum
        if count >= BUFSIZE:
            h.work_in = work.link
            return self.qpkt(work)
        dev = h.device_in
        if dev is None:
            return self.wait()
        h.device_in = dev.link
        dev.datum = work.data[count]
        work.datum = count + 1
        return self.qpkt(dev)


class IdleTask(Task):
    def fn(self, pkt, i):
        i.count -= 1
        if i.count == 0:
            return self.hold()
        if i.control & 1 == 0:
            i.control = i.control // 2
            return self.release(I_DEVA)
        i.control = (i.control // 2) ^ 0xD008
        return self.release(I_DEVB)


class WorkTask(Task):
    def fn(self, pkt, w):
        if pkt is None:
            return self.wait()
        if w.destination == I_HANDLERA:
            dest = I_HANDLERB
        else:
            dest = I_HANDLERA
        w.destination = dest
        pkt.ident = dest
        pkt.datum = 0
        for i in range(BUFSIZE):
            w.count += 1
            if w.count > 26:
                w.count = 1
            pkt.data[i] = 65 + w.count - 1
        return self.qpkt(pkt)


def schedule():
    t = area.task_list
    while t is not None:
        if t.holding_or_waiting():
            t = t.link
        else:
            t = t.run()


def run_once():
    area.tasks = [None] * 10
    area.task_list = None
    area.hold_count = 0
    area.qpkt_count = 0

    IdleTask(I_IDLE, 1, None, False, False, False, IdleRec())

    wkq = Packet(None, 0, K_WORK)
    wkq = Packet(wkq, 0, K_WORK)
    WorkTask(I_WORK, 1000, wkq, True, True, False, WorkerRec())

    wkq = Packet(None, I_DEVA, K_DEV)
    wkq = Packet(wkq, I_DEVA, K_DEV)
    wkq = Packet(wkq, I_DEVA, K_DEV)
    HandlerTask(I_HANDLERA, 2000, wkq, True, True, False, HandlerRec())

    wkq = Packet(None, I_DEVB, K_DEV)
    wkq = Packet(wkq, I_DEVB, K_DEV)
    wkq = Packet(wkq, I_DEVB, K_DEV)
    HandlerTask(I_HANDLERB, 3000, wkq, True, True, False, HandlerRec())

    DeviceTask(I_DEVA, 4000, None, False, True, False, DeviceRec())
    DeviceTask(I_DEVB, 5000, None, False, True, False, DeviceRec())

    schedule()
    print(area.hold_count, area.qpkt_count)


def main():
    n = int(sys.argv[1])
    for _ in range(n):
        run_once()


if __name__ == "__main__":
    main()
