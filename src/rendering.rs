use super::*;

/// Textures keep the same orientation in every block position.
pub fn block_variant(_x: i32, _y: i32, _z: i32) -> u32 {
    0
}

/// Get upright UV coordinates in the atlas for one block face.
/// Each atlas cell holds a tile plus a border of repeated edge pixels.
/// Returns (u_min, v_min, u_max, v_max) in normalized coordinates [0, 1]
pub fn block_uv(block: u8, face: usize, _variant: u32) -> ([f32; 2], [f32; 2], [f32; 2], [f32; 2]) {
    // Map block type to atlas tile position
    // Special case: GRASS has different top and side textures
    let tile_id = block_texture(block, face);

    let ([uv_min_x, uv_min_y], [uv_max_x, uv_max_y]) = texture_layout::tile_uv_rect(tile_id);

    // Base UV corners: (bottom-left, bottom-right, top-right, top-left)
    let base_uv0 = [uv_min_x, uv_min_y];
    let base_uv1 = [uv_max_x, uv_min_y];
    let base_uv2 = [uv_max_x, uv_max_y];
    let base_uv3 = [uv_min_x, uv_max_y];

    // PNG/atlas V=0 is the top row. Map top vertices to the top of the
    // texture on every side, accounting for each face's vertex winding.
    match face {
        0 | 5 => (base_uv3, base_uv0, base_uv1, base_uv2),
        1 | 4 => (base_uv3, base_uv2, base_uv1, base_uv0),
        _ => (base_uv0, base_uv1, base_uv2, base_uv3),
    }
}

/// Get shading multiplier based on face direction for depth perception
pub fn face_shade(face: usize) -> f32 {
    if face == 3 {
        0.72
    } else if face == 0 || face == 1 {
        0.86
    } else {
        1.0
    }
}

/// Create an empty mesh (used when chunk has no visible faces)
pub fn empty_mesh() -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<[f32; 2]>::new());
    mesh.insert_indices(Indices::U32(Vec::new()));
    mesh
}

fn append_quad_indices(base: u32, indices_u16: &mut Vec<u16>, indices_u32: &mut Option<Vec<u32>>) {
    let quad = [base, base + 1, base + 2, base, base + 2, base + 3];
    if let Some(indices) = indices_u32 {
        indices.extend_from_slice(&quad);
    } else if quad[5] <= u16::MAX as u32 {
        indices_u16.extend_from_slice(&[
            base as u16,
            (base + 1) as u16,
            (base + 2) as u16,
            base as u16,
            (base + 2) as u16,
            (base + 3) as u16,
        ]);
    } else {
        let mut indices = Vec::with_capacity(indices_u16.len() + quad.len());
        indices.extend(indices_u16.drain(..).map(u32::from));
        indices.extend_from_slice(&quad);
        *indices_u32 = Some(indices);
    }
}

/// Build complete mesh for a chunk.
/// Reads blocks in storage order and emits visible faces without revisiting
/// the chunk once for each face direction.
pub fn build_chunk_mesh(world: &WorldData, coord: IVec3) -> Mesh {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices_u16 = Vec::<u16>::new();
    let mut indices_u32 = None;
    let Some(chunk) = world.chunks.get(&coord) else {
        return empty_mesh();
    };

    // Chunk storage is X-contiguous, so this loop reads it sequentially.
    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x, y, z);
                let block = chunk.get(local);
                if block == AIR {
                    continue;
                }

                let block_pos = coord * CHUNK_SIZE + local;
                let variant = block_variant(block_pos.x, block_pos.y, block_pos.z);
                if is_cross_plant(block) {
                    let origin = local.as_vec3();
                    let planes = [
                        (
                            [
                                origin,
                                origin + Vec3::new(1.0, 0.0, 1.0),
                                origin + Vec3::new(1.0, 1.0, 1.0),
                                origin + Vec3::Y,
                            ],
                            Vec3::new(-1.0, 0.0, 1.0).normalize(),
                        ),
                        (
                            [
                                origin + Vec3::X,
                                origin + Vec3::Z,
                                origin + Vec3::new(0.0, 1.0, 1.0),
                                origin + Vec3::new(1.0, 1.0, 0.0),
                            ],
                            Vec3::new(1.0, 0.0, 1.0).normalize(),
                        ),
                    ];
                    let (uv0, uv1, uv2, uv3) = block_uv(block, 2, variant);
                    for (corners, normal) in planes {
                        for reverse in [false, true] {
                            let ordered = if reverse {
                                [corners[3], corners[2], corners[1], corners[0]]
                            } else {
                                corners
                            };
                            let base = positions.len() as u32;
                            for (corner, uv) in ordered.iter().zip([uv0, uv1, uv2, uv3].iter()) {
                                positions.push(corner.to_array());
                                normals.push(normal.to_array());
                                uvs.push(*uv);
                            }
                            append_quad_indices(base, &mut indices_u16, &mut indices_u32);
                        }
                    }
                    continue;
                }
                for face_index in 0..6 {
                    let axis = face_index / 2;
                    let positive = face_index % 2 == 0;
                    let mut normal = IVec3::ZERO;
                    normal[axis] = if positive { 1 } else { -1 };

                    let crosses_chunk_boundary = if positive {
                        local[axis] == CHUNK_SIZE - 1
                    } else {
                        local[axis] == 0
                    };
                    let neighbor_block = if crosses_chunk_boundary {
                        super::block_at(world, block_pos + normal)
                    } else {
                        chunk.get(local + normal)
                    };
                    // Only a full opaque cube hides the face behind it: plants,
                    // leaves and torches have see-through pixels, and fluids
                    // must not hide the solid face behind them. Faces between
                    // two blocks of the same see-through kind are skipped too.
                    if block_occludes(neighbor_block) || neighbor_block == block {
                        continue;
                    }

                    let u_axis = (axis + 1) % 3;
                    let v_axis = (axis + 2) % 3;
                    let mut origin = local;
                    origin[axis] += if positive { 1 } else { 0 };

                    let mut u_direction = IVec3::ZERO;
                    u_direction[u_axis] = 1;
                    let mut v_direction = IVec3::ZERO;
                    v_direction[v_axis] = 1;

                    let origin = origin.as_vec3();
                    let u_direction = u_direction.as_vec3();
                    let v_direction = v_direction.as_vec3();
                    let corners = if positive {
                        [
                            origin,
                            origin + u_direction,
                            origin + u_direction + v_direction,
                            origin + v_direction,
                        ]
                    } else {
                        [
                            origin,
                            origin + v_direction,
                            origin + u_direction + v_direction,
                            origin + u_direction,
                        ]
                    };

                    let (uv0, uv1, uv2, uv3) = block_uv(block, face_index, variant);
                    let _shade = face_shade(face_index);
                    let base = positions.len() as u32;
                    for (corner, uv) in corners.iter().zip([uv0, uv1, uv2, uv3].iter()) {
                        positions.push(corner.to_array());
                        normals.push(normal.as_vec3().to_array());
                        uvs.push(*uv);
                    }
                    append_quad_indices(base, &mut indices_u16, &mut indices_u32);
                }
            }
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(match indices_u32 {
        Some(indices) => Indices::U32(indices),
        None => Indices::U16(indices_u16),
    });
    mesh
}
