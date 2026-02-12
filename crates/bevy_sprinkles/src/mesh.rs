use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, MeshVertexAttributeId, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
};

use crate::asset::{ParticleMesh, QuadOrientation};

fn create_cylinder_mesh(
    top_radius: f32,
    bottom_radius: f32,
    height: f32,
    radial_segments: u32,
    rings: u32,
    cap_top: bool,
    cap_bottom: bool,
) -> Mesh {
    let radial_segments = radial_segments.max(3);
    let rings = rings.max(1);
    let half_height = height / 2.0;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let side_normal_y = (bottom_radius - top_radius) / height;
    let side_normal_scale = 1.0 / (1.0 + side_normal_y * side_normal_y).sqrt();

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let y = half_height - height * v;
        let radius = top_radius + (bottom_radius - top_radius) * v;

        for segment in 0..=radial_segments {
            let u = segment as f32 / radial_segments as f32;
            let theta = u * std::f32::consts::TAU;
            let (sin_theta, cos_theta) = theta.sin_cos();

            let x = cos_theta * radius;
            let z = sin_theta * radius;

            positions.push([x, y, z]);

            let nx = cos_theta * side_normal_scale;
            let ny = side_normal_y * side_normal_scale;
            let nz = sin_theta * side_normal_scale;
            normals.push([nx, ny, nz]);

            uvs.push([u, v]);
        }
    }

    let verts_per_ring = radial_segments + 1;
    for ring in 0..rings {
        for segment in 0..radial_segments {
            let top_left = ring * verts_per_ring + segment;
            let top_right = ring * verts_per_ring + segment + 1;
            let bottom_left = (ring + 1) * verts_per_ring + segment;
            let bottom_right = (ring + 1) * verts_per_ring + segment + 1;

            indices.push(top_left);
            indices.push(top_right);
            indices.push(bottom_left);

            indices.push(top_right);
            indices.push(bottom_right);
            indices.push(bottom_left);
        }
    }

    generate_cap(
        cap_top,
        top_radius,
        half_height,
        1.0,
        radial_segments,
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
    );

    generate_cap(
        cap_bottom,
        bottom_radius,
        -half_height,
        -1.0,
        radial_segments,
        &mut positions,
        &mut normals,
        &mut uvs,
        &mut indices,
    );

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn generate_cap(
    enabled: bool,
    radius: f32,
    y: f32,
    normal_y: f32,
    radial_segments: u32,
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
) {
    if !enabled || radius <= 0.0 {
        return;
    }

    let center_index = positions.len() as u32;

    positions.push([0.0, y, 0.0]);
    normals.push([0.0, normal_y, 0.0]);
    uvs.push([0.5, 0.5]);

    for segment in 0..=radial_segments {
        let u = segment as f32 / radial_segments as f32;
        let theta = u * std::f32::consts::TAU;
        let (sin_theta, cos_theta) = theta.sin_cos();

        positions.push([cos_theta * radius, y, sin_theta * radius]);
        normals.push([0.0, normal_y, 0.0]);
        uvs.push([cos_theta * 0.5 + 0.5, sin_theta * 0.5 + 0.5]);
    }

    for segment in 0..radial_segments {
        let first = center_index + 1 + segment;
        let second = center_index + 1 + segment + 1;
        if normal_y > 0.0 {
            indices.push(center_index);
            indices.push(second);
            indices.push(first);
        } else {
            indices.push(center_index);
            indices.push(first);
            indices.push(second);
        }
    }
}

fn create_prism_mesh(left_to_right: f32, size: Vec3, subdivide: Vec3) -> Mesh {
    let start_pos = size * -0.5;
    let subdivide_w = subdivide.x as usize;
    let subdivide_h = subdivide.y as usize;
    let subdivide_d = subdivide.z as usize;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    {
        let mut y = start_pos.y;
        let mut thisrow: u32 = 0;
        let mut prevrow: u32 = 0;

        for j in 0..=(subdivide_h + 1) {
            let scale = j as f32 / (subdivide_h + 1) as f32;
            let scaled_size_x = size.x * scale;
            let start_x = start_pos.x + (1.0 - scale) * size.x * left_to_right;

            let v = j as f32 / (2.0 * (subdivide_h + 1) as f32);

            let mut x = 0.0_f32;
            for i in 0..=(subdivide_w + 1) {
                let u = (i as f32 / (subdivide_w + 1) as f32) * scale;

                positions.push([start_x + x, -y, -start_pos.z]);
                normals.push([0.0, 0.0, 1.0]);
                uvs.push([u, v]);

                positions.push([start_x + scaled_size_x - x, -y, start_pos.z]);
                normals.push([0.0, 0.0, -1.0]);
                uvs.push([u, v]);

                if i > 0 && j == 1 {
                    let i2 = (i * 2) as u32;

                    indices.extend_from_slice(&[thisrow + i2 - 2, thisrow + i2, prevrow + i2]);
                    indices.extend_from_slice(&[
                        thisrow + i2 - 1,
                        thisrow + i2 + 1,
                        prevrow + i2 + 1,
                    ]);
                } else if i > 0 && j > 0 {
                    let i2 = (i * 2) as u32;

                    indices.extend_from_slice(&[
                        thisrow + i2 - 2,
                        prevrow + i2,
                        prevrow + i2 - 2,
                        thisrow + i2 - 2,
                        thisrow + i2,
                        prevrow + i2,
                    ]);
                    indices.extend_from_slice(&[
                        thisrow + i2 - 1,
                        prevrow + i2 + 1,
                        prevrow + i2 - 1,
                        thisrow + i2 - 1,
                        thisrow + i2 + 1,
                        prevrow + i2 + 1,
                    ]);
                }

                x += scale * size.x / (subdivide_w + 1) as f32;
            }

            y += size.y / (subdivide_h + 1) as f32;
            prevrow = thisrow;
            thisrow = positions.len() as u32;
        }
    }

    {
        let normal_left = Vec3::new(-size.y, size.x * left_to_right, 0.0).normalize();
        let normal_right = Vec3::new(size.y, size.x * (1.0 - left_to_right), 0.0).normalize();

        let mut y = start_pos.y;
        let mut thisrow = positions.len() as u32;
        let mut prevrow: u32 = 0;

        for j in 0..=(subdivide_h + 1) {
            let scale = j as f32 / (subdivide_h + 1) as f32;
            let left = start_pos.x + (size.x * (1.0 - scale) * left_to_right);
            let right = left + (size.x * scale);

            let v = j as f32 / (2.0 * (subdivide_h + 1) as f32);

            let mut z = start_pos.z;
            for i in 0..=(subdivide_d + 1) {
                let u = i as f32 / (subdivide_d + 1) as f32;

                positions.push([right, -y, -z]);
                normals.push(normal_right.to_array());
                uvs.push([u, v]);

                positions.push([left, -y, z]);
                normals.push(normal_left.to_array());
                uvs.push([u, v]);

                if i > 0 && j > 0 {
                    let i2 = (i * 2) as u32;

                    indices.extend_from_slice(&[
                        thisrow + i2 - 2,
                        prevrow + i2,
                        prevrow + i2 - 2,
                        thisrow + i2 - 2,
                        thisrow + i2,
                        prevrow + i2,
                    ]);
                    indices.extend_from_slice(&[
                        thisrow + i2 - 1,
                        prevrow + i2 + 1,
                        prevrow + i2 - 1,
                        thisrow + i2 - 1,
                        thisrow + i2 + 1,
                        prevrow + i2 + 1,
                    ]);
                }

                z += size.z / (subdivide_d + 1) as f32;
            }

            y += size.y / (subdivide_h + 1) as f32;
            prevrow = thisrow;
            thisrow = positions.len() as u32;
        }
    }

    {
        let mut z = start_pos.z;
        let mut thisrow = positions.len() as u32;
        let mut prevrow: u32 = 0;

        for j in 0..=(subdivide_d + 1) {
            let v = j as f32 / (2.0 * (subdivide_d + 1) as f32);

            let mut x = start_pos.x;
            for i in 0..=(subdivide_w + 1) {
                let u = i as f32 / (subdivide_w + 1) as f32;

                positions.push([x, start_pos.y, -z]);
                normals.push([0.0, -1.0, 0.0]);
                uvs.push([u, v]);

                if i > 0 && j > 0 {
                    let curr = thisrow + i as u32;
                    let prev_curr = prevrow + i as u32;

                    indices.extend_from_slice(&[
                        curr - 1,
                        prev_curr,
                        prev_curr - 1,
                        curr - 1,
                        curr,
                        prev_curr,
                    ]);
                }

                x += size.x / (subdivide_w + 1) as f32;
            }

            z += size.z / (subdivide_d + 1) as f32;
            prevrow = thisrow;
            thisrow = positions.len() as u32;
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn create_subdivided_quad(size: Vec2, subdivisions_x: u32, subdivisions_y: u32) -> Mesh {
    let cols = subdivisions_x + 1;
    let rows = subdivisions_y + 1;
    let vertex_count = ((cols + 1) * (rows + 1)) as usize;
    let index_count = (cols * rows * 6) as usize;

    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut indices = Vec::with_capacity(index_count);

    let half_w = size.x * 0.5;
    let half_d = size.y * 0.5;

    for iy in 0..=rows {
        let ty = iy as f32 / rows as f32;
        let y = -half_d + ty * size.y;
        for ix in 0..=cols {
            let tx = ix as f32 / cols as f32;
            let x = -half_w + tx * size.x;
            positions.push([x, y, 0.0]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([tx, 1.0 - ty]);
        }
    }

    let stride = cols + 1;
    for iy in 0..rows {
        for ix in 0..cols {
            let i00 = iy * stride + ix;
            let i10 = i00 + 1;
            let i01 = i00 + stride;
            let i11 = i01 + 1;
            indices.push(i00);
            indices.push(i10);
            indices.push(i11);
            indices.push(i00);
            indices.push(i11);
            indices.push(i01);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn create_base_mesh(config: &ParticleMesh) -> Mesh {
    match config {
        ParticleMesh::Quad {
            orientation,
            size,
            subdivide,
        } => {
            let subdivisions_x = subdivide.x as u32;
            let subdivisions_y = subdivide.y as u32;
            let mut mesh = create_subdivided_quad(*size, subdivisions_x, subdivisions_y);

            let rotation = match orientation {
                QuadOrientation::FaceZ => None,
                QuadOrientation::FaceX => Some(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2)),
                QuadOrientation::FaceY => Some(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
            };

            if let Some(rot) = rotation {
                if let Some(VertexAttributeValues::Float32x3(positions)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
                {
                    for pos in positions.iter_mut() {
                        let v = rot * Vec3::from_array(*pos);
                        *pos = v.to_array();
                    }
                }
                if let Some(VertexAttributeValues::Float32x3(normals)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
                {
                    for normal in normals.iter_mut() {
                        let v = rot * Vec3::from_array(*normal);
                        *normal = v.to_array();
                    }
                }
            }

            mesh
        }
        ParticleMesh::Sphere { radius } => Mesh::from(Sphere::new(*radius)),
        ParticleMesh::Cuboid { half_size } => Mesh::from(Cuboid::new(
            half_size.x * 2.0,
            half_size.y * 2.0,
            half_size.z * 2.0,
        )),
        ParticleMesh::Cylinder {
            top_radius,
            bottom_radius,
            height,
            radial_segments,
            rings,
            cap_top,
            cap_bottom,
        } => create_cylinder_mesh(
            *top_radius,
            *bottom_radius,
            *height,
            *radial_segments,
            *rings,
            *cap_top,
            *cap_bottom,
        ),
        ParticleMesh::Prism {
            left_to_right,
            size,
            subdivide,
        } => create_prism_mesh(*left_to_right, *size, *subdivide),
    }
}

pub(crate) fn create_particle_mesh(
    config: &ParticleMesh,
    particle_count: u32,
    meshes: &mut Assets<Mesh>,
) -> Handle<Mesh> {
    let base_mesh = create_base_mesh(config);

    let base_positions: Vec<[f32; 3]> = extract_float32x3(
        &base_mesh,
        Mesh::ATTRIBUTE_POSITION,
    )
    .unwrap_or_default();

    let base_normals: Vec<[f32; 3]> = extract_float32x3(
        &base_mesh,
        Mesh::ATTRIBUTE_NORMAL,
    )
    .unwrap_or_else(|| vec![[0.0, 0.0, 1.0]; base_positions.len()]);

    let base_uvs: Vec<[f32; 2]> = base_mesh
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .and_then(|attr| match attr {
            VertexAttributeValues::Float32x2(v) => Some(v.clone()),
            _ => None,
        })
        .unwrap_or_else(|| vec![[0.0, 0.0]; base_positions.len()]);

    let base_indices: Vec<u32> = base_mesh
        .indices()
        .map(|indices| indices.iter().map(|i| i as u32).collect())
        .unwrap_or_else(|| (0..base_positions.len() as u32).collect());

    let vertices_per_mesh = base_positions.len();
    let indices_per_mesh = base_indices.len();

    let total_vertices = particle_count as usize * vertices_per_mesh;
    let total_indices = particle_count as usize * indices_per_mesh;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(total_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(total_vertices);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(total_vertices);
    let mut uv_bs: Vec<[f32; 2]> = Vec::with_capacity(total_vertices);
    let mut indices: Vec<u32> = Vec::with_capacity(total_indices);

    for particle_idx in 0..particle_count {
        let base_vertex = (particle_idx as usize * vertices_per_mesh) as u32;
        let particle_index_f32 = particle_idx as f32;

        for i in 0..vertices_per_mesh {
            positions.push(base_positions[i]);
            normals.push(base_normals[i]);
            uvs.push(base_uvs[i]);
            uv_bs.push([particle_index_f32, 0.0]);
        }

        for &idx in &base_indices {
            indices.push(base_vertex + idx);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, uv_bs);
    mesh.insert_indices(Indices::U32(indices));

    meshes.add(mesh)
}

fn extract_float32x3(
    mesh: &Mesh,
    attribute: impl Into<MeshVertexAttributeId>,
) -> Option<Vec<[f32; 3]>> {
    mesh.attribute(attribute).and_then(|attr| match attr {
        VertexAttributeValues::Float32x3(v) => Some(v.clone()),
        _ => None,
    })
}
