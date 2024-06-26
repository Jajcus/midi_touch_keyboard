#!/usr/bin/python3

import os
import re
import sys

from dataclasses import dataclass
from typing import List, IO, Optional


class Error(Exception):
    def __init__(self, message: str):
        self.message = message
        self.args = (message,)
    def __str__(self):
        return self.message


def main():
    os.chdir("fab-nc")

    try:
        process_mill("front.ngc", "front.nc")
        process_mill("back.ngc", "back.nc")
        if os.path.exists("drill.ngc"):
            process_drill("drill.ngc", "drill-{tool.diameter}.nc")
        if os.path.exists("milldrill.ngc"):
            process_mill("milldrill.ngc", "milldrill.nc", tool_type='mill')
        process_mill("outline.ngc", "outline.nc")
    except Error as err:
        print(str(err), file=sys.stderr)
        sys.exit(1)


@dataclass
class Preamble:
    commands: List[str]


@dataclass
class Tool:
    type: str
    diameter: float


@dataclass
class Block:
    commands: List[str]
    last: bool
    radius: Optional[float] = None


def process_mill(src_filename, dst_filename, tool_type=None):
    with open(src_filename) as src:
        try:
            with open(dst_filename, "w") as dst:
                process_mill_files(src, dst, tool_type=tool_type)
        except:
            if os.path.exists(dst_filename):
                os.unlink(dst_filename)
            raise


def process_drill(src_filename, dst_filename_pat):
    with open(src_filename) as src:
        preamble = load_preamble(src)
        while True:
            tool = load_toolchange(src)
            if tool is None:
                break
            block = load_block(src)
            if not block or block.last:
                break
            with open(dst_filename_pat.format(tool=tool), "w") as dst:
                write_preamble(src.name, dst, preamble, tool)
                for block in split_block(block):
                    write_block(dst, block, tool)
                write_postamble(dst)

PREAMBLE_PASS_RE = re.compile(r"^(?P<pass>G(?:94|21|90|91.1|01 F\d+(?:\.\d+)?))(?: *\([^)]*\))?$")
TOOL_CHANGE_START_RE = re.compile(r"^G00 Z.*(Retract[^)]*)")
SPINDLE_RE = re.compile(r"^(?:G00 +)?S(?P<speed>\d+)(?: *\([^)]*\))?$")
COMMENT_RE = re.compile(r"^(?: *\((?P<comment>[^)]*)\))+ *")


def process_mill_files(src: IO[str], dst: IO[str], tool_type=None):
    preamble = load_preamble(src)

    tool = load_toolchange(src)

    if tool_type is not None and tool is not None:
        # need to override tool type in 'milldrill' file
        tool.type = tool_type

    write_preamble(src.name, dst, preamble, tool)

    while True:
        block = load_block(src)
        if not block:
            break
        write_block(dst, block, tool)
        if block.last:
            break

    write_postamble(dst)


def load_preamble(src: IO[str]) -> Preamble:
    """Read preamble generated by pcb2gcode.

    pass-through known commands. Fix setting the spindle speed."""

    commands = []
    while True:
        line = src.readline()
        if line is None or "(Retract to tool change height)" in line:
            break
        line = line.strip()
        if not line:
            continue
        match = PREAMBLE_PASS_RE.match(line)
        if match:
            commands.append(match.group("pass"))
            continue
        match = SPINDLE_RE.match(line)
        if match:
            commands.append("M3 S" + match.group("speed"))
            continue
        match = COMMENT_RE.match(line)
        if match:
            continue
        match = TOOL_CHANGE_START_RE.match(line)
        if match:
            break
        raise Error(f"Unexpected preamble line: {line!r}")

    return Preamble(commands=commands)

def write_preamble(src_filename: str, dst: IO[str], preamble: Preamble, tool: Optional[Tool]):

    dst.write(f"(created by postprocess.py from {src_filename!r})\n")
    dst.write("\n")

    if tool:
        dst.write(f"(Use {tool.diameter:.2f}mm {tool.type} bit)\n")
        dst.write("\n")

    for line in preamble.commands:
        dst.write(line + "\n")

TOOL_RE = re.compile(r"^.*\(MSG, Change tool bit to (?P<type>mill|drill)"
                     r" (?:diameter|size) (?P<diameter>\d+(?:\.\d+)?) ?mm.*\)")

def load_toolchange(src) -> Optional[Tool]:
    """Skip over tool change code generated by pcb2gcode.

    Return the tool diameter extracted from a comment."""
    beginning = True
    tool_type = None
    tool_diameter = None
    while True:
        line = src.readline()
        if line is None:
            break
        line = line.strip()
        if not line:
            if beginning:
                continue
            else:
                break
        beginning = False
        if line.startswith("G01"):
            raise Error(f"Missed the end of tool change")

        match = TOOL_RE.match(line)
        if match:
            tool_type = match.group("type")
            tool_diameter = float(match.group("diameter"))

    if tool_type and tool_diameter:
        return Tool(type=tool_type, diameter=tool_diameter)
    else:
        return None


BLOCK_PASS_RE = re.compile(r"^(?P<pass>G(?:0?0|0?1|0?4)[^(]*)(?: *\([^)]*\))?$")
G2_RE = re.compile(r"^(?P<pass>G0?2 [^)]*[IJ](?P<radius1>-?\d+(?:\.\d+)?) [IJ](?P<radius2>-?\d+(?:\.\d+)?)[^)]*)(?: *\([^)]*\))?")


def load_block(src) -> Optional[Block]:
    """Process a single block.

    Return True if it was a proper block, return False if it is the end of the program."""

    commands = []
    has_g1 = False
    beginning = True
    radius = 0.0

    while True:
        line = src.readline()
        if line is None:
            break
        line = line.strip()
        if not line:
            if beginning:
                continue
            else:
                break
        beginning = False
        match = BLOCK_PASS_RE.match(line)
        if match:
            cmd = match.group("pass")
            commands.append(cmd)
            if cmd.startswith("G01") or cmd.startswith("G1 "):
                has_g1 = True
            continue
        match = G2_RE.match(line)
        if match:
            cmd = match.group("pass")
            commands.append(cmd)
            radius1 = abs(float(match.group("radius1") or 0.0))
            radius2 = abs(float(match.group("radius2") or 0.0))
            radius = max(radius, radius1, radius2)
            continue

        match = COMMENT_RE.match(line)
        if match:
            continue
        raise Error(f"Unexpected block line: {line!r}")

    if not commands:
        return None

    if radius > 0.0:
        radius = radius
    else:
        radius = None

    return Block(commands=commands, last=not has_g1, radius=radius)


FEED_RE = re.compile(r"G0?1 F\d+(?:\.\d+)(?: *\([^)]*\))?$")


def split_block(block: Block) -> List[Block]:
    if block.last:
        return [block]
    blocks = []
    commands = block.commands
    if FEED_RE.match(commands[0]):
        common = [commands[0]]
        commands = commands[1:]
    else:
        common = []

    newblock = None
    for command in commands:
        if command.startswith("G0 ") or command.startswith("G00"):
            if newblock:
                blocks.append(newblock)
            newblock = Block(commands=common + [command], last=False)
        elif not newblock:
            raise Error(f"Unexpected initial command in block: {command!r}")
        else:
            newblock.commands.append(command)

    if newblock:
        blocks.append(newblock)

    return blocks


def write_block(dst: IO[str], block: Block, tool: Optional[Tool]):
    dst.write("\n")
    if block.last:
        block_name = "pre-finish"
    elif tool is not None:
        block_name = tool.type
    else:
        block_name = "block"
    if block.radius:
        diameter = block.radius * 2
        if tool and tool.diameter:
            diameter += tool.diameter
        block_name = f"{block_name} d={diameter:.2f}"
    dst.write(f"(Block-name: {block_name})\n")
    dst.write("(Block-expand: 0)\n")
    dst.write("(Block-enable: 1)\n")

    for line in block.commands:
        dst.write(line + "\n")


def write_postamble(dst):
    dst.write("\n")
    dst.write("(Block-name: finish)\n")
    dst.write("(Block-expand: 0)\n")
    dst.write("(Block-enable: 1)\n")
    dst.write("M5\n")
    dst.write("M2\n")


if __name__ == "__main__":
    main()
