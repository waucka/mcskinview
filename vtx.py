#!/usr/bin/env python3

import os.path as path
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

def load_mesh(mesh_filename):
    vertices = {}
    joints = {}

    mesh = Collada(mesh_filename)
    nodes_dict = {}

    for node in mesh.scenes[0].nodes:
        if type(node.children[0]) is not GeometryNode:
            continue
        nodes_dict[node.children[0].geometry.id] = node.id

    for geom in mesh.scenes[0].objects('geometry'):
        # This check is probably redundant now, but I want to
        # avoid pulling in extraneous stuff.
        if geom.original.id in nodes_dict:
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
    for node in mesh.scenes[0].nodes:
        if node.id.endswith('_bone'):
            mat = node.matrix
            joints[node.id] = [mat[0][3],
                               mat[1][3],
                               mat[2][3]]
    return vertices, joints

rust_common = '''
extern crate nalgebra;
#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 3],
    pub texcoord: [f32; 2],
    pub normal: [f32; 3],
}

implement_vertex!(Vertex, position, texcoord, normal);
'''

def generate_rust(ostream, common_mod, all_vertices, joints):
    ostream.write("extern crate nalgebra;\nuse {0}::Vertex;".format(common_mod))
    for piece_name, vertices in all_vertices.items():
        ostream.write('\n')
        ostream.write("pub const {0}: &'static [Vertex] = &[\n".format(piece_name.upper()))
        for vtx in vertices:
            ostream.write("    Vertex {{ position: [{x:.4f}, {y:.4f}, {z:.4f}],  texcoord: [{s:.4f}, {t:.4f}],  normal: [{nx:.4f}, {ny:.4f}, {nz:.4f}] }},\n".format(**vtx.__dict__))
        ostream.write('    ];\n')

    for joint_name, joint in joints.items():
        ostream.write('\n')
        ostream.write("pub const {joint_name}: &'static nalgebra::Vec3<f32> = &nalgebra::Vec3{{ x: {x:.4f}, y: {y:.4f}, z: {z:.4f} }};\n".format(joint_name=joint_name.upper(),
                                                                                                                                                 x=joint[0],
                                                                                                                                                 y=joint[1],
                                                                                                                                                 z=joint[2]))

def list_pieces(mesh_filename):
    mesh = Collada(mesh_filename)
    import ipdb; ipdb.set_trace()
    for node in mesh.scenes[0].nodes:
        if type(node.children[0]) is not GeometryNode:
            continue
        print(node.id)


def main():
    parser = argparse.ArgumentParser(description='Extract vertices from a Collada file and convert them to Rust code')
    parser.add_argument('--list-pieces', dest='list_pieces', action='store_true',
                        help='list pieces available in mesh')
    parser.add_argument('-m', '--mesh', dest='meshes', metavar='MESH:OUTPUT', action='append',
                        help='specify input and output files (input:output)')
    parser.add_argument('-c', '--common', dest='common', type=str,
                        help='write common definitions to file')

    args = parser.parse_args()

    if args.list_pieces:
        list_pieces(args.mesh)
        sys.exit(0)

    if args.common is None:
        parser.error('-c is required.')
        sys.exit(1)

    with open(args.common, 'w') as f:
        f.write(rust_common)

    common_filename, _ = path.splitext(args.common);
    common_mod = path.basename(common_filename)

    for io_pair in args.meshes:
        parts = io_pair.split(':')
        vertices, joints = load_mesh(parts[0])
        with open(parts[1], 'w') as f:
            generate_rust(f, common_mod, vertices, joints)

if __name__ == '__main__':
    main()
