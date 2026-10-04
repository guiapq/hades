//! Utilitário de inspeção de assets para o cliente Berenice.

use berenice::vfs::GrfArchive;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();
    let grf_path = std::env::var("HADES_GRF_PATH").unwrap_or_else(|_| "data.grf".to_string());
    println!("Carregando GRF configurado em HADES_GRF_PATH: {}", grf_path);
    let grf = GrfArchive::open(&grf_path)?;
    println!("Total de arquivos no GRF: {}", grf.file_count());

    let mut spr_count = 0;
    let mut act_count = 0;
    let mut rsm_count = 0;
    let mut rsw_count = 0;
    let mut gnd_count = 0;
    let mut gat_count = 0;
    let mut bmp_count = 0;

    let mut prontera_models = Vec::new();
    let mut player_sprites = Vec::new();

    for name in grf.file_names() {
        if name.ends_with(".spr") {
            spr_count += 1;
            if (name.contains("novice")
                || name.contains("body")
                || name.contains("head")
                || name.contains("poring"))
                && player_sprites.len() < 10
            {
                player_sprites.push(name.clone());
            }
        } else if name.ends_with(".act") {
            act_count += 1;
        } else if name.ends_with(".rsm") {
            rsm_count += 1;
            if name.contains("prontera") && prontera_models.len() < 10 {
                prontera_models.push(name.clone());
            }
        } else if name.ends_with(".rsw") {
            rsw_count += 1;
        } else if name.ends_with(".gnd") {
            gnd_count += 1;
        } else if name.ends_with(".gat") {
            gat_count += 1;
        } else if name.ends_with(".bmp") || name.ends_with(".tga") || name.ends_with(".png") {
            bmp_count += 1;
        }
    }

    println!("Estatísticas de Assets:");
    println!("  .spr (Sprites 2D): {}", spr_count);
    println!("  .act (Ações/Animações): {}", act_count);
    println!("  .rsm (Modelos 3D): {}", rsm_count);
    println!("  .rsw (Mundos 3D): {}", rsw_count);
    println!("  .gnd (Chão 3D): {}", gnd_count);
    println!("  .gat (Colisão): {}", gat_count);
    println!("  Imagens (.bmp/.tga/.png): {}", bmp_count);

    println!("\nTestando armas específicas para 검사_남:");
    for sword in &[
        "data/sprite/인간족/검사/검사_남_검.act",
        "data/sprite/인간족/검사/검사_남_검.spr",
        "data/sprite/인간족/검사/검사_남_양손검.act",
        "data/sprite/인간족/검사/검사_남_양손검.spr",
        "data/sprite/인간족/검사/검사_남_도끼.act",
        "data/sprite/인간족/검사/검사_남_도끼.spr",
        "data/sprite/인간족/검사/검사_남_단검.act",
        "data/sprite/인간족/검사/검사_남_단검.spr",
        "data/sprite/방패/검사/검사_남_방패.act",
        "data/sprite/방패/검사/검사_남_방패.spr",
        "data/sprite/인간족/검사/검사_남_방패.act",
        "data/sprite/인간족/검사/검사_남_방패.spr",
    ] {
        println!("  {} -> existe? {}", sword, grf.contains(sword));
    }

    println!("\nTestando parse_spr em 남_본헬름 e 검사_남_검:");
    if let Some(bytes) = grf.extract("data/sprite/악세사리/남/남_본헬름.spr") {
        match hades_ro_prere::spr_parser::parse_spr(&bytes) {
            Ok(spr) => println!("  남_본헬름.spr -> OK! Frames: {}", spr.frames.len()),
            Err(e) => println!("  남_본헬름.spr -> Erro: {:?}", e),
        }
    }
    for act_path in &[
        "data/sprite/악세사리/남/남_본헬름.act",
        "data/sprite/인간족/검사/검사_남_검.act",
    ] {
        if let Some(bytes) = grf.extract(act_path) {
            if let Ok(act) = hades_ro_prere::act_parser::parse_act(&bytes) {
                println!("\n[{}] Total actions: {}", act_path, act.actions.len());
                for a in [0, 8, 32] {
                    if a < act.actions.len() {
                        if let Some(f0) = act.actions[a].frames.first() {
                            println!("  Action {:02} Frame 0: clips={}, attach_points={:?}", a, f0.clips.len(), f0.attach_points);
                        }
                    }
                }
            }
        }
    }
    if let Some(bytes) = grf.extract("data/sprite/인간족/검사/검사_남_양손검.spr") {
        println!("  검사_남_양손검.spr: tamanho={}", bytes.len());
        if bytes.len() >= 8 {
            println!("  Magic: {:?}", &bytes[0..2]);
            println!("  Versão: 0x{:02x}{:02x}", bytes[3], bytes[2]);
            println!("  Frames indexados: {}", u16::from_le_bytes([bytes[4], bytes[5]]));
            println!("  Frames RGBA: {}", u16::from_le_bytes([bytes[6], bytes[7]]));
        }
    }





    for act_path in &[
        "data/sprite/인간족/몸통/남/검사_남.act",
        "data/sprite/인간족/검사/검사_남_검.act",
        "data/sprite/인간족/검사/검사_남_양손검.act",
    ] {
        if let Some(bytes) = grf.extract(act_path) {
            if let Ok(act) = hades_ro_prere::act_parser::parse_act(&bytes) {
                println!("\n=== {} ===", act_path);
                for g in 0..(act.actions.len() / 8) {
                    let a = g * 8;
                    let action = &act.actions[a];
                    let visible_clips: usize = action.frames.iter().map(|f| f.clips.iter().filter(|c| c.spr_index >= 0).count()).sum();
                    println!("  Grupo {:02} (Ação {:03}..{:03}): frames={}, delay_ms={}, total_visible_clips={}", g, a, a + 7, action.frames.len(), action.delay_ms, visible_clips);
                }
            }
        }
    }


    let head_act_path = "data/sprite/인간족/머리통/남/1_남.act";
    if let Some(bytes) = grf.extract(head_act_path) {
        let sub_ver = bytes[2];
        let main_ver = bytes[3];
        let ver = main_ver as f32 + (sub_ver as f32) / 10.0;
        let mut pos = 16;
        for a in 0..16 {
            let frame_count = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
            pos += 4;
            for _f in 0..frame_count {
                pos += 32;
                let clip_count = u32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                pos += 4;
                for _ in 0..clip_count {
                    pos += 16;
                    if ver >= 2.0 {
                        pos += 8;
                        if ver > 2.3 { pos += 4; }
                        pos += 8;
                        if ver >= 2.5 { pos += 8; }
                    }
                }
                if ver >= 2.0 { pos += 4; }
                if ver >= 2.3 {
                    let attach_count = i32::from_le_bytes(bytes[pos..pos+4].try_into().unwrap()) as usize;
                    pos += 4 + attach_count * 16;
                }
            }
            let label = if a < 8 { "Idle" } else { "Walk" };
            println!("Head Action {:02} [{} Dir {}]: {} frames", a, label, a % 8, frame_count);
        }
    }


    if let Some(shadow_bytes) = grf.extract("data/sprite/shadow.spr") {
        println!("\n[shadow.spr] Tamanho: {} bytes", shadow_bytes.len());
        println!("  Magic: {:?}", &shadow_bytes[0..2]);
        let ver = u16::from_le_bytes(shadow_bytes[2..4].try_into().unwrap());
        let indexed_count = u16::from_le_bytes(shadow_bytes[4..6].try_into().unwrap());
        let rgba_count = u16::from_le_bytes(shadow_bytes[6..8].try_into().unwrap());
        println!(
            "  Versão: 0x{:04X}, Frames Indexados: {}, Frames RGBA: {}",
            ver, indexed_count, rgba_count
        );

        let w = u16::from_le_bytes(shadow_bytes[8..10].try_into().unwrap()) as usize;
        let h = u16::from_le_bytes(shadow_bytes[10..12].try_into().unwrap()) as usize;
        println!("  Frame 0: {}x{} = {} pixels esperados", w, h, w * h);

        // Palette no final: 1024 bytes
        let pal_offset = shadow_bytes.len() - 1024;
        let palette = &shadow_bytes[pal_offset..];
        println!(
            "  Paleta[0..4]: RGBA = ({}, {}, {}, {})",
            palette[0], palette[1], palette[2], palette[3]
        );
        println!(
            "  Paleta[4..8]: RGBA = ({}, {}, {}, {})",
            palette[4], palette[5], palette[6], palette[7]
        );

        // RLE decode
        let mut pixels = Vec::with_capacity(w * h);
        let mut cursor = 12;
        while pixels.len() < w * h && cursor < pal_offset {
            let b = shadow_bytes[cursor];
            cursor += 1;
            if b == 0 {
                if cursor < pal_offset {
                    let count = shadow_bytes[cursor] as usize;
                    cursor += 1;
                    let to_add = count.min(w * h - pixels.len());
                    pixels.extend(std::iter::repeat_n(0u8, to_add));
                }
            } else {
                pixels.push(b);
            }
        }
        println!(
            "  Pixels decodificados: {} / {} (cursor parou em {} / paleta em {})",
            pixels.len(),
            w * h,
            cursor,
            pal_offset
        );
    }

    println!("\nBuscando sprites em data/sprite/npc/:");
    let mut npc_sprites = Vec::new();
    for name in grf.file_names() {
        if name.starts_with("data/sprite/npc/") && name.ends_with(".spr") {
            npc_sprites.push(name.clone());
            if npc_sprites.len() >= 10 {
                break;
            }
        }
    }
    for s in &npc_sprites {
        println!("  - {}", s);
    }

    if let Some(rsw_bytes) = grf.extract("data/prontera.rsw") {
        println!(
            "\n[prontera.rsw] Extraído com sucesso! Tamanho: {} bytes",
            rsw_bytes.len()
        );
        let major = rsw_bytes[4];
        let minor = rsw_bytes[5];
        println!("  Versão RSW: {}.{}", major, minor);

        // Busca strings com .rsm no arquivo inteiro
        let mut rsm_refs = Vec::new();
        let needle = b".rsm";
        let mut i = 0;
        while i + 4 <= rsw_bytes.len() {
            if &rsw_bytes[i..i + 4] == needle {
                // Retrocede para o início da string
                let mut start = i;
                while start > 0 && rsw_bytes[start - 1] >= 32 && rsw_bytes[start - 1] < 127 {
                    start -= 1;
                }
                let s = String::from_utf8_lossy(&rsw_bytes[start..i + 4]).to_string();
                if !rsm_refs.contains(&s) {
                    rsm_refs.push(s);
                }
            }
            i += 1;
        }
        println!("  Modelos 3D (.rsm) únicos referenciados em prontera.rsw: {}", rsm_refs.len());
        for (idx, r) in rsm_refs.iter().take(10).enumerate() {
            println!("    [{}] {}", idx, r);
        }
    }

    if let Some(rsm_bytes) = grf.extract("data/model/prontera/flowerpot_01.rsm") {
        match hades_ro_prere::rsm_parser::parse_rsm(&rsm_bytes) {
            Ok(model) => {
                println!("\n[flowerpot_01.rsm] Decodificado com sucesso via parse_rsm!");
                println!("  Versão: {}.{}, Texturas: {:?}", model.major, model.minor, model.textures);
                println!("  Nós: {}, Vértices Nó 0: {}, Faces Nó 0: {}", model.nodes.len(), model.nodes[0].vertices.len(), model.nodes[0].faces.len());
            }
            Err(e) => println!("\n[flowerpot_01.rsm] Erro de parse: {:?}", e),
        }
    }

    if let Some(rsm_bytes) = grf.extract("data/model/prontera/prt_k_bunsu_1.rsm") {
        let len = rsm_bytes.len();
        println!("\n[prt_k_bunsu_1.rsm] Últimos 20 bytes (0x{:04x}..0x{:04x}):", len - 20, len);
        println!("{:02x?}", &rsm_bytes[len - 20..len]);
    }

    if let Some(rsm_bytes) = grf.extract("data/model/prontera/prt_k_bunsu_1.rsm") {
        match hades_ro_prere::rsm_parser::parse_rsm(&rsm_bytes) {
            Ok(model) => {
                println!("\n[prt_k_bunsu_1.rsm] Decodificado com sucesso via parse_rsm!");
                println!("  Versão: {}.{}, Texturas: {:?}", model.major, model.minor, model.textures);
                println!("  Nós: {}, Vértices Nó 0: {}, Faces Nó 0: {}", model.nodes.len(), model.nodes[0].vertices.len(), model.nodes[0].faces.len());
            }
            Err(e) => println!("\n[prt_k_bunsu_1.rsm] Erro de parse: {:?}", e),
        }
    }

    if let Some(rsw_bytes) = grf.extract("data/prontera.rsw") {
        match hades_ro_prere::rsw_parser::parse_rsw(&rsw_bytes) {
            Ok(scene) => {
                println!("\n[prontera.rsw] Parse com sucesso via parse_rsw!");
                println!("  Versão: {}.{}, GND: {}, GAT: {}", scene.major, scene.minor, scene.gnd_file, scene.gat_file);
                println!("  Total de modelos 3D encontrados: {}", scene.models.len());

                let mut nearby = Vec::new();
                for (idx, m) in scene.models.iter().enumerate() {
                    let world_x = 156.0 + m.position[0];
                    let world_y = 196.0 + m.position[2];
                    let dist = ((world_x - 156.0).powi(2) + (world_y - 180.0).powi(2)).sqrt();
                    if dist <= 25.0 {
                        nearby.push((idx, m, world_x, world_y, dist));
                    }
                }
                println!("  Modelos em um raio de 25 células da praça central (156, 180): {}", nearby.len());
                for (idx, m, wx, wy, dist) in nearby.iter().take(10) {
                    println!("    [{}] File: '{}' | Nome: '{}' | World: ({:.1}, {:.1}) | Dist: {:.1}", idx, m.filename, m.name, wx, wy, dist);
                }
            }
            Err(e) => println!("\n[prontera.rsw] Erro ao parsear: {:?}", e),
        }
    }

    if let Some(rsm_bytes) = grf.extract("data/model/prontera/flowerpot_01.rsm") {
        println!("\n[flowerpot_01.rsm] Bytes[0..64]:\n{:02x?}", &rsm_bytes[0..64]);
    }

    if let Some(shadow_bytes) = grf.extract("data/sprite/shadow.spr") {
        let shadow_spr = hades_ro_prere::spr_parser::parse_spr(&shadow_bytes)?;
        println!(
            "\n[shadow_spr] Decodificado com sucesso! Versão: 0x{:04X}, Frames: {}",
            shadow_spr.version,
            shadow_spr.frames.len()
        );
        println!(
            "  Frame 0: {}x{} pixels",
            shadow_spr.frames[0].width, shadow_spr.frames[0].height
        );
    }

    if let Some(npc_bytes) = grf.extract("data/sprite/npc/4_m_fairysoldier2.spr") {
        let npc_spr = hades_ro_prere::spr_parser::parse_spr(&npc_bytes)?;
        println!(
            "\n[4_m_fairysoldier2.spr] Decodificado com sucesso! Versão: 0x{:04X}, Frames: {}",
            npc_spr.version,
            npc_spr.frames.len()
        );
        println!(
            "  Frame 0: {}x{} pixels",
            npc_spr.frames[0].width, npc_spr.frames[0].height
        );
    }

    println!("\nExemplos de Modelos 3D (.rsm) encontrados no GRF:");
    let mut count = 0;
    for m in grf.file_names() {
        if m.ends_with(".rsm") {
            println!("  - {}", m);
            count += 1;
            if count >= 15 { break; }
        }
    }

    // Verifica se prontera.rsw existe
    println!("\nVerificando arquivos chave de Prontera:");
    println!("  data/prontera.gat: {}", grf.contains("data/prontera.gat"));
    println!("  data/prontera.rsw: {}", grf.contains("data/prontera.rsw"));
    println!("  data/prontera.gnd: {}", grf.contains("data/prontera.gnd"));

    Ok(())
}
