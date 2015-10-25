#!/usr/bin/env python3

import sys
import decimal
import argparse

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

def load_mesh(mesh_filename, pieces):
    vertices = {}

    mesh = Collada(mesh_filename)
    nodes_dict = {}

    for node in mesh.scenes[0].nodes:
        if type(node.children[0]) is not GeometryNode:
            continue
        #import ipdb; ipdb.set_trace()
        if pieces is None or node.id in pieces:
            nodes_dict[node.children[0].geometry.id] = node.id

    for geom in mesh.scenes[0].objects('geometry'):
        if geom.original.id in nodes_dict:
            #print(geom.original.id)
            prims = list(geom.primitives())
            assert(len(prims) == 1)
            assert(len(prims[0].vertex_index) == len(prims[0].normal_index))
            assert(len(prims[0].normal_index) == len(prims[0].texcoord_indexset[0]))
            piece_vertices = []
            for tri in prims[0].triangleset():
                for i in range(0, 3):
                    piece_vertices.append(Vertex(tri.vertices[i][0], tri.vertices[i][1], tri.vertices[i][2],
                                                 tri.texcoords[0][i][0], tri.texcoords[0][i][1],
                                                 tri.normals[i][0], tri.normals[i][1], tri.normals[i][2]))
            vertices[nodes_dict[geom.original.id]] = piece_vertices
    return vertices

def generate_c(ostream, all_vertices):
    ostream.write('''typedef struct {
  GLfloat x;
  GLfloat y;
  GLfloat z;

  GLfloat s;
  GLfloat t;

  GLfloat nx;
  GLfloat ny;
  GLfloat nz;
};
''')
    for piece_name, vertices in all_vertices.items():
        ostream.write('\n')
        ostream.write("vertex_data_t[] {0} = {\n".format(piece_name))
        vtx_strs = []
        for vtx in vertices:
            vtx_strs.append("  {{ {x:.4f}f, {y:.4f}f, {z:.4f}f,  {s:.4f}f, {t:.4f}f,  {nx:.4f}f, {ny:.4f}f, {nz:.4f}f }}".format(**vtx.__dict__))
        ostream.write(',\n'.join(vtx_strs))
        ostream.write('\n};\n')

def generate_rust(ostream, all_vertices):
    ostream.write('''#[derive(Copy, Clone)]
pub struct Vertex {
    position: [f32; 3],
    texcoord: [f32; 2],
    normal: [f32; 3],
}

implement_vertex!(Vertex, position, texcoord, normal);
''');

    for piece_name, vertices in all_vertices.items():
        ostream.write('\n')
        ostream.write("pub const {0}: &'static [Vertex] = &[\n".format(piece_name.upper()))
        for vtx in vertices:
            ostream.write("    Vertex {{ position: [{x:.4f}, {y:.4f}, {z:.4f}],  texcoord: [{s:.4f}, {t:.4f}],  normal: [{nx:.4f}, {ny:.4f}, {nz:.4f}] }},\n".format(**vtx.__dict__))
        ostream.write('    ];\n')

dispatch = {
    'c': generate_c,
    'rust': generate_rust,
}

def list_pieces(mesh_filename):
    mesh = Collada(mesh_filename)
    for node in mesh.scenes[0].nodes:
        if type(node.children[0]) is not GeometryNode:
            continue
        print(node.id)


def main():
    parser = argparse.ArgumentParser(description='Process some integers.')
    parser.add_argument('--cc', dest='lang', action='store_const',
                        const='c',
                        help='output C code')
    parser.add_argument('--rust', dest='lang', action='store_const',
                        const='rust',
                        help='output Rust code')
    parser.add_argument('--list-pieces', dest='list_pieces', action='store_true',
                        help='list pieces available in mesh')
    parser.add_argument('-p', '--piece', dest='pieces', action='append')
    parser.add_argument('-o', '--output', dest='output', type=str)
    parser.add_argument(dest='mesh', metavar='MESH', type=str,
                        help='mesh to load')

    args = parser.parse_args()

    if args.lang is None:
        if args.list_pieces:
            list_pieces(args.mesh)
            sys.exit(0)
        parser.error("Use either --cc or --rust.")
        sys.exit(1)
    vertices = load_mesh(args.mesh, args.pieces)

    if args.output is not None:
        with open(args.output, 'w') as f:
            dispatch[args.lang](f, vertices)
    else:
        dispatch[args.lang](sys.stdout, vertices)

if __name__ == '__main__':
    main()
