#!/usr/bin/env python3

import sys
import numpy as np
import decimal
from collada import Collada
from collada.scene import GeometryNode

decimal.getcontext().prec = 3

def clamp(x):
    return decimal.Decimal(str(x))

class Vertex(object):
    def __init__(self, x, y, z, s, t, nx, ny, nz):
        self.x = clamp(x)
        self.y = clamp(y)
        self.z = clamp(z)

        self.s = clamp(s)
        self.t = clamp(t)

        self.nx = clamp(nx)
        self.ny = clamp(ny)
        self.nz = clamp(nz)

vertices = []

mesh = Collada('steve.dae')
nodes_dict = {}

for node in mesh.scenes[0].nodes:
    if type(node.children[0]) is not GeometryNode:
        continue
    if len(sys.argv) < 3:
        print(node.id)
    assert(len(node.children) == 1)
    nodes_dict[node.id] = node.children[0].geometry.id

if len(sys.argv) < 3:
    sys.exit(0)

target_nodes = set([nodes_dict[x] for x in sys.argv[2:]])
for geom in mesh.scenes[0].objects('geometry'):
    if geom.original.id in target_nodes:
        #print(geom.original.id)
        prims = list(geom.primitives())
        assert(len(prims) == 1)
        assert(len(prims[0].vertex_index) == len(prims[0].normal_index))
        assert(len(prims[0].normal_index) == len(prims[0].texcoord_indexset[0]))
        for tri in prims[0].triangleset():
            for i in range(0, 3):
                vertices.append(Vertex(tri.vertices[i][0], tri.vertices[i][1], tri.vertices[i][2],
                                       tri.texcoords[0][i][0], tri.texcoords[0][i][1],
                                       tri.normals[i][0], tri.normals[i][1], tri.normals[i][2]))

        # for i in range(0, len(prims[0].vertex_index)):
        #     print("{0} {1} {2}".format(prims[0].vertex_index[i],
        #                                prims[0].texcoord_indexset[0][i],
        #                                prims[0].normal_index[i]))
        #     vtx = prims[0].vertex[prims[0].vertex_index[i]]
        #     tc = prims[0].texcoordset[0][prims[0].texcoord_indexset[0][i]]
        #     nm = prims[0].normal[prims[0].normal_index[i]]

        #     vertices.append(Vertex(vtx[0], vtx[1], vtx[2],
        #                            tc[0], tc[1],
        #                            nm[0], nm[1], nm[2]))

def generate_c():
    print('''typedef struct {
  GLfloat x;
  GLfloat y;
  GLfloat z;

  GLfloat s;
  GLfloat t;

  GLfloat nx;
  GLfloat ny;
  GLfloat nz;
};''')
    print()
    print('vertex_data_t[] vertices = {')
    vtx_strs = []
    for vtx in vertices:
        vtx_strs.append("  {{ {x:.4f}f, {y:.4f}f, {z:.4f}f,  {s:.4f}f, {t:.4f}f,  {nx:.4f}f, {ny:.4f}f, {nz:.4f}f }}".format(**vtx.__dict__))
    print(',\n'.join(vtx_strs))
    print('};')

def generate_rust():
    print('''#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    texcoord: [f32; 2],
    normal: [f32; 3],
}

implement_vertex!(Vertex, position, texcoord, normal);
''');

    print("pub const VERTICES: &'static [Vertex] = &[")
    for vtx in vertices:
        print("    Vertex {{ position: [{x:.4f}, {y:.4f}, {z:.4f}],  texcoord: [{s:.4f}, {t:.4f}],  normal: [{nx:.4f}, {ny:.4f}, {nz:.4f}] }},".format(**vtx.__dict__))
    print('    ];')

dispatch = {
    'c': generate_c,
    'rust': generate_rust,
}

def main():
    dispatch[sys.argv[1]]()

if __name__ == '__main__':
    main()
