#!/usr/bin/env python3
"""Generate Geofront's original two-material relay beacon GLB, no dependencies."""
import json
import struct
from pathlib import Path

# Each cuboid gets hard normals; dimensions are metres before in-game scale.
# Geometry is inscribed in a 0.56 m square, so it does not mask the combat grid.
shell = [
    ((0, .025, 0), (.56, .050, .56)),
    ((0, .060, 0), (.45, .035, .45)),
    ((0, .124, 0), (.25, .130, .25)),
    ((0, .205, 0), (.20, .040, .20)),
]
signal = [((0, .169, 0), (.30, .030, .30))]
for direction in (-1, 1):
    signal.extend([
        ((direction * .19, .089, 0), (.065, .020, .21)),
        ((0, .089, direction * .19), (.21, .020, .065)),
    ])

def mesh(boxes):
    pos, normal, indices = [], [], []
    for center, size in boxes:
        for axis in range(3):
            u, v = (axis + 1) % 3, (axis + 2) % 3
            for side in (-1, 1):
                first = len(pos)
                for du, dv in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
                    p = list(center)
                    p[axis] += side * size[axis] / 2
                    p[u] += du * size[u] / 2
                    p[v] += dv * size[v] / 2
                    pos.append(p)
                    n = [0, 0, 0]
                    n[axis] = side
                    normal.append(n)
                order = (0, 1, 2, 0, 2, 3) if side > 0 else (0, 2, 1, 0, 3, 2)
                indices.extend(first + i for i in order)
    return pos, normal, indices

data = bytearray()
views, accessors = [], []

def append(values, fmt, gltf_type, target, bounds=False):
    while len(data) % 4:
        data.append(0)
    offset = len(data)
    for value in values:
        data.extend(struct.pack(fmt, *value) if isinstance(value, (tuple, list)) else struct.pack(fmt, value))
    view = len(views)
    views.append(dict(buffer=0, byteOffset=offset, byteLength=len(data)-offset, target=target))
    desc = dict(bufferView=view, componentType=5123 if gltf_type == 'SCALAR' else 5126,
                count=len(values), type=gltf_type)
    if bounds:
        desc.update(min=list(map(min, zip(*values))), max=list(map(max, zip(*values))))
    accessors.append(desc)
    return len(accessors) - 1

primitives = []
for material, boxes in enumerate((shell, signal)):
    pos, normal, elements = mesh(boxes)
    primitives.append(dict(
        attributes=dict(POSITION=append(pos, '<3f', 'VEC3', 34962, True),
                        NORMAL=append(normal, '<3f', 'VEC3', 34962)),
        indices=append(elements, '<H', 'SCALAR', 34963), material=material))
gltf = dict(
    asset=dict(version='2.0', generator='Geofront relay beacon generator'),
    scene=0, scenes=[dict(nodes=[0])], nodes=[dict(mesh=0)],
    meshes=[dict(name='Relay beacon', primitives=primitives)],
    materials=[
        dict(name='Ceramic graphite', pbrMetallicRoughness=dict(
            baseColorFactor=[.065, .078, .09, 1], metallicFactor=0, roughnessFactor=.74)),
        dict(name='Signal panels', pbrMetallicRoughness=dict(
            baseColorFactor=[.75, .84, .90, 1], metallicFactor=0, roughnessFactor=.35)),
    ],
    buffers=[dict(byteLength=len(data))], bufferViews=views, accessors=accessors,
)
encoded = json.dumps(gltf, separators=(',', ':')).encode()
encoded += b' ' * (-len(encoded) % 4)
data.extend(b'\0' * (-len(data) % 4))
out = Path(__file__).resolve().parents[1] / 'assets/models/geofront/relay-beacon.glb'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_bytes(struct.pack('<4sII', b'glTF', 2, 28 + len(encoded) + len(data))
                + struct.pack('<I4s', len(encoded), b'JSON') + encoded
                + struct.pack('<I4s', len(data), b'BIN\0') + data)
