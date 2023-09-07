use std::{env, vec, fs};
use std::path::{PathBuf, Path};
use rusqlite::{Connection, Result, params, Transaction, Statement, Rows};
use crate::define;
use crate::db_models::{ProbeLog, DataSetItem, MapInfo, MapNode, MapEdge, MapLayout, MapData, ProbeStat, UserProbeData, UserHostGroup, UserHostTag, UserHost, UserService};
use rushmap_core::option;
use rushmap_core::result::{PortScanResult, HostScanResult, PingResult, TracerouteResult, PingResponse};

pub fn init(handle: tauri::AppHandle) {
    copy_db_from_resource(handle);
}

pub fn copy_db_from_resource(handle: tauri::AppHandle) {
    let resource_path = handle.path_resolver()
    .resolve_resource(format!("resources/{}", crate::define::DB_NAME))
    .expect("failed to resolve resource");
    let mut path: PathBuf = env::current_exe().unwrap();
    path.pop();
    path.push(crate::define::DB_NAME);

    if resource_path.exists() && !path.exists() {
        match fs::copy(resource_path, path) {
            Ok(_) => println!("Database copied successfully"),
            Err(e) => println!("Error copying database: {}", e),
        }
    }
}

pub fn copy_db() {
    let mut src_path: PathBuf = env::current_exe().unwrap();
    src_path.pop();
    src_path.push(Path::new("resources").join(crate::define::DB_NAME));
    let mut dst_path: PathBuf = env::current_exe().unwrap();
    dst_path.pop();
    dst_path.push(crate::define::DB_NAME);
    
    if src_path.exists() && !dst_path.exists() {
        match fs::copy(src_path, dst_path) {
            Ok(_) => println!("Database copied successfully"),
            Err(e) => println!("Error copying database: {}", e),
        }
    }
}

pub fn connect_db() -> Result<Connection,rusqlite::Error> {
    let mut path: PathBuf = env::current_exe().unwrap();
    path.pop();
    path.push(define::DB_NAME);
    if !path.exists() {
        copy_db();
    }
    let conn = Connection::open(path)?;
    Ok(conn)
}

pub fn insert_port_scan_result(conn:&Connection, probe_id: String, scan_result: PortScanResult, option_value: String) -> Result<usize,rusqlite::Error> {
    let node = scan_result.nodes[0].clone();
    let mut affected_row_count: usize = 0;
    let sql: &str = "INSERT INTO probe_result (
        probe_id, 
        probe_type_id,
        probe_target_addr,
        probe_target_name,
        protocol_id,
        probe_option,
        issued_at)  
        VALUES (?1,?2,?3,?4,?5,?6, datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        probe_id,
        option::CommandType::PortScan.id(),
        node.ip_addr.to_string(),
        node.host_name,
        option::IpNextLevelProtocol::TCP.id(),
        option_value
    ];
    match conn.execute(sql, params_vec) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => return Err(e),
    };

    let sql: &str = "INSERT INTO host_scan_result (probe_id, ip_addr, host_name, port, protocol_id, mac_addr, vendor, os_name, cpe, issued_at)
    VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        probe_id,
        node.ip_addr.to_string(),
        node.host_name,
        0,
        option::IpNextLevelProtocol::TCP.id(),
        node.mac_addr,
        node.vendor_info,
        node.os_name,
        node.cpe
    ];   
    match conn.execute(sql, params_vec) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => return Err(e),
    };

    for port in node.services.clone() {
        let sql: &str = "INSERT INTO port_scan_result (probe_id, socket_addr, ip_addr, host_name, port, port_status_id, protocol_id, service_id, service_version, issued_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,datetime(CURRENT_TIMESTAMP, 'localtime'));";
        let params_vec: &[&dyn rusqlite::ToSql] = params![
            probe_id,
            format!("{}:{}",node.ip_addr, port.port_number),
            node.ip_addr.to_string(),
            node.host_name,
            port.port_number,
            port.port_status.name(),
            option::IpNextLevelProtocol::TCP.id(),
            port.service_name,
            port.service_version
        ];   
        match conn.execute(sql, params_vec) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => return Err(e),
        };
    }
    Ok(affected_row_count)
}

pub fn insert_host_scan_result(conn:&Connection, probe_id: String, scan_result: HostScanResult, option_value: String)  -> Result<usize,rusqlite::Error> {
    let mut affected_row_count: usize = 0;
    let sql: &str = "INSERT INTO probe_result (
        probe_id, 
        probe_type_id,
        probe_target_addr,
        probe_target_name,
        protocol_id,
        probe_option,
        issued_at)  
        VALUES (?1,?2,?3,?4,?5,?6,datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        probe_id,
        option::CommandType::HostScan.id(),
        String::new(),
        String::new(),
        scan_result.protocol.id(),
        option_value
    ];   
    match conn.execute(sql, params_vec) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => return Err(e),
    };
    for host in scan_result.nodes {
        let sql: &str = "INSERT INTO host_scan_result (probe_id, ip_addr, host_name, port, protocol_id, mac_addr, vendor, os_name, issued_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,datetime(CURRENT_TIMESTAMP, 'localtime'));";
        let params_vec: &[&dyn rusqlite::ToSql] = params![
            probe_id,
            host.ip_addr.to_string(),
            host.host_name,
            if scan_result.protocol == option::IpNextLevelProtocol::ICMPv4 || scan_result.protocol == option::IpNextLevelProtocol::ICMPv6 {0} else { host.services[0].port_number },
            scan_result.protocol.id(),
            host.mac_addr,
            host.vendor_info,
            host.os_name
        ];   
        match conn.execute(sql, params_vec) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => return Err(e),
        };
    }

    Ok(affected_row_count)
}

pub fn insert_ping_result(conn:&Connection, probe_id: String, ping_result: PingResult, option_value: String)  -> Result<usize,rusqlite::Error> {
    let mut affected_row_count: usize = 0;
    
    let ping_response: PingResponse = if ping_result.stat.responses.len() > 0 {ping_result.stat.responses[0].clone()}else{PingResponse::new()};
    let sql: &str = "INSERT INTO probe_result (
        probe_id, 
        probe_type_id,
        probe_target_addr,
        probe_target_name,
        protocol_id,
        probe_option, 
        issued_at)  
        VALUES (?1,?2,?3,?4,?5,?6,datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        probe_id,
        option::CommandType::Ping.id(),
        ping_response.ip_addr.to_string(),
        ping_response.host_name,
        ping_response.protocol,
        option_value
    ];   
    match conn.execute(sql, params_vec) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => return Err(e),
    };
    // insert ping_stat
    let sql: &str = "INSERT INTO ping_stat (probe_id, ip_addr, host_name, transmitted_count, received_count, min_rtt, avg_rtt, max_rtt, issued_at)
    VALUES (?1,?2,?3,?4,?5,?6,?7,?8,datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        probe_id,
        ping_response.ip_addr.to_string(),
        ping_response.host_name,
        ping_result.stat.transmitted_count,
        ping_result.stat.received_count,
        ping_result.stat.min,
        ping_result.stat.avg,
        ping_result.stat.max
    ];
    match conn.execute(sql, params_vec) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => return Err(e),
    };
    for ping in ping_result.stat.responses {
        let sql: &str = "INSERT INTO ping_result (probe_id, seq, ip_addr, host_name, port, port_status_id, ttl, hop, rtt, issued_at) 
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,datetime(CURRENT_TIMESTAMP, 'localtime'));";
        let params_vec: &[&dyn rusqlite::ToSql] = params![
            probe_id,
            ping.seq,
            ping.ip_addr.to_string(),
            ping.host_name,
            ping.port_number.unwrap_or(0),
            ping.status.name(),
            ping.ttl,
            ping.hop,
            ping.rtt
        ];   
        match conn.execute(sql, params_vec) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => return Err(e),
        };
    }
    Ok(affected_row_count)
}

pub fn insert_trace_result(conn:&Connection, probe_id: String, trace_result: TracerouteResult, option_value: String)  -> Result<usize,rusqlite::Error> {
    let mut affected_row_count: usize = 0;
    let first_node:PingResponse = if trace_result.nodes.len() > 0 {trace_result.nodes[trace_result.nodes.len() - 1].clone()}else{ PingResponse::new() };
    let sql: &str = "INSERT INTO probe_result (
        probe_id, 
        probe_type_id,
        probe_target_addr,
        probe_target_name,
        protocol_id,
        probe_option,
        issued_at)  
        VALUES (?1,?2,?3,?4,?5,?6,datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        probe_id,
        option::CommandType::Traceroute.id(),
        first_node.ip_addr.to_string(),
        first_node.host_name,
        option::IpNextLevelProtocol::UDP.id(),
        option_value
    ];   
    match conn.execute(sql, params_vec) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => return Err(e),
    };
    for node in trace_result.nodes {
        let sql: &str = "INSERT INTO traceroute_result (probe_id, seq, ip_addr, host_name, ttl, hop, rtt, node_type, issued_at)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,datetime(CURRENT_TIMESTAMP, 'localtime'));";
        let params_vec: &[&dyn rusqlite::ToSql] = params![
            probe_id,
            node.seq,
            node.ip_addr.to_string(),
            node.host_name,
            node.ttl,
            node.hop,
            node.rtt,
            node.node_type.name()
        ];   
        match conn.execute(sql, params_vec) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => return Err(e),
        };
    }
    Ok(affected_row_count)
}

pub fn get_probe_result(target_host: String, probe_types: Vec<String>, start_date: String, end_date: String) -> Vec<ProbeLog> {
    let target_host: String = if target_host.is_empty(){String::from("%")}else{if crate::validator::is_valid_hostname(target_host.clone()) {target_host} else {String::from("%")}} ;
    let mut results: Vec<ProbeLog> = vec![];
    let conn = connect_db().unwrap();
    let mut in_params: String = String::new();
    let mut pram_index: usize = 4;
    for _t in probe_types.clone() {
        pram_index += 1;
        if pram_index == 5 {
            in_params = format!("?{}", pram_index);
        }else{
            in_params = format!("{}, ?{}", in_params, pram_index);
        }
    }
    let mut sql: String = "SELECT A.id, A.probe_id, A.probe_type_id, B.probe_type_name, A.probe_target_addr, A.probe_target_name, A.protocol_id, A.probe_option, A.issued_at 
    FROM probe_result AS A INNER JOIN probe_type AS B ON A.probe_type_id = B.probe_type_id ".to_string();
    sql = format!("{} WHERE A.issued_at BETWEEN ?1 AND ?2 ", sql);
    sql = format!("{} AND (A.probe_target_addr LIKE ?3 OR A.probe_target_name LIKE ?4) ", sql);
    sql = format!("{} AND A.probe_type_id IN ({}) ", sql, in_params);
    sql = format!("{} ORDER BY A.issued_at DESC;", sql);
    let mut stmt = conn.prepare(sql.as_str()).unwrap();
    let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![
            &start_date,
            &end_date,
            &target_host,
            &target_host
        ]; 
    for t in &probe_types {
        params_vec.push(t);
    }
    let result_iter = stmt.query_map(&params_vec[..], |row| {
        Ok(ProbeLog {
            id: row.get(0).unwrap(), 
            probe_id: row.get(1).unwrap(), 
            probe_type_id: row.get(2).unwrap(), 
            probe_type_name: row.get(3).unwrap(), 
            probe_target_addr: row.get(4).unwrap(), 
            probe_target_name: row.get(5).unwrap(), 
            protocol_id: row.get(6).unwrap(), 
            probe_option: row.get(7).unwrap(), 
            issued_at: row.get(8).unwrap() 
        })
    }).unwrap();
    for result in result_iter {
        results.push(result.unwrap());
    }
    return results;
}

pub fn get_probed_hosts() -> Vec<DataSetItem> {
    let mut results: Vec<DataSetItem> = vec![];
    let conn = connect_db().unwrap();
    let sql: &str = "SELECT DISTINCT probe_target_addr, probe_target_name FROM probe_result WHERE probe_target_addr IS NOT NULL AND probe_target_addr <> '' ORDER BY probe_target_addr ASC;";
    let mut stmt = conn.prepare(sql).unwrap();
    let result_iter = stmt.query_map([], |row| {
        Ok(DataSetItem{id: row.get(0).unwrap(), name: row.get(1).unwrap()})
    }).unwrap();
    for result in result_iter {
        match result {
            Ok(r) => {
                results.push(r);
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    let mut dataset: Vec<DataSetItem> = vec![];
    let rows = results.clone();
    for item in results {
        let count = rows.iter().filter(|&row| *row.id == item.id).count();
        if count > 1 {
            let count = rows.iter().filter(|&row| *row.id == item.id && *row.id != *row.name && *row.name != "".to_owned()).count();
            if count > 0 {
                if item.name.is_empty() || item.id == item.name {
                    continue;
                }else{
                    dataset.push(item);
                }
            }else{
                dataset.push(item);
            }
        }else{
            dataset.push(item);
        }
    }
    for row in &mut dataset {
        if row.name.is_empty() {
            row.name = row.id.clone();
        }
    }
    return dataset;
}

#[allow(unused)]
pub fn get_map_list() -> Vec<MapInfo> {
    let mut results: Vec<MapInfo> = vec![];
    let conn = connect_db().unwrap();
    let sql: &str = "SELECT map_id, map_name, display_order, created_at FROM map_info ORDER BY display_order ASC;";
    let mut stmt = conn.prepare(sql).unwrap();
    let result_iter = stmt.query_map([], |row| {
        Ok(MapInfo{
            map_id: row.get(0).unwrap(), 
            map_name: row.get(1).unwrap(), 
            display_order: row.get(2).unwrap(),
            created_at: row.get(3).unwrap()
        })
    }).unwrap();
    for result in result_iter {
        match result {
            Ok(r) => {
                results.push(r);
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    results
}

pub fn get_map_info(map_id: u32) -> Option<MapInfo> {
    let conn = connect_db().unwrap();
    let sql: &str = "SELECT map_id, map_name, display_order, created_at FROM map_info WHERE map_id = ?1;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];   
    let mut stmt = conn.prepare(sql).unwrap();
    let result_iter = stmt.query_map(params_vec, |row| {
        Ok(MapInfo{
            map_id: row.get(0).unwrap(), 
            map_name: row.get(1).unwrap(), 
            display_order: row.get(2).unwrap(),
            created_at: row.get(3).unwrap()
        })
    }).unwrap();
    for result in result_iter {
        match result {
            Ok(r) => {
                return Some(r);
            },
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
    None
}

pub fn insert_map_info(tran:&Transaction, model: MapInfo) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "INSERT INTO map_info (map_id, map_name, display_order, created_at) 
        VALUES (?1,?2,?3,datetime(CURRENT_TIMESTAMP, 'localtime'));";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.map_name,
        model.display_order
    ];   
    tran.execute(sql, params_vec)
}

#[allow(unused)]
pub fn delete_map_info(tran:&Transaction, map_id: u32) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_info WHERE map_id = ?1;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn insert_map_node(tran:&Transaction, model: MapNode) -> Result<usize,rusqlite::Error> {
    let sql: &str = "INSERT INTO map_node (map_id, node_id, node_name, ip_addr, host_name) 
        VALUES (?1,?2,?3,?4,?5);";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.node_id,
        model.node_name,
        model.ip_addr,
        model.host_name
    ];   
    tran.execute(sql, params_vec)
}

#[allow(unused)]
pub fn delete_map_node(tran:&Transaction, model: MapNode) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_node WHERE map_id = ?1 AND node_id = ?2;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.node_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn delete_map_nodes(tran:&Transaction, map_id: u32) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_node WHERE map_id = ?1;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn insert_map_edge(tran:&Transaction, model: MapEdge) -> Result<usize,rusqlite::Error> {
    let sql: &str = "INSERT INTO map_edge (map_id, edge_id, source_node_id, target_node_id, edge_label) 
        VALUES (?1,?2,?3,?4,?5);";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.edge_id,
        model.source_node_id,
        model.target_node_id,
        model.edge_label
    ];   
    tran.execute(sql, params_vec)
}

#[allow(unused)]
pub fn delete_map_edge(tran:&Transaction, model: MapEdge) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_edge WHERE map_id = ?1 AND edge_id = ?2;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.edge_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn delete_map_edges(tran:&Transaction, map_id: u32) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_edge WHERE map_id = ?1;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn insert_map_layout(tran:&Transaction, model: MapLayout) -> Result<usize,rusqlite::Error> {
    let sql: &str = "INSERT INTO map_layout (map_id, node_id, x_value, y_value) 
        VALUES (?1,?2,?3,?4);";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.node_id,
        model.x_value,
        model.y_value
    ];   
    tran.execute(sql, params_vec)
}

#[allow(unused)]
pub fn delete_map_layout(tran:&Transaction, model: MapLayout) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_layout WHERE map_id = ?1 AND node_id = ?2;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        model.map_id,
        model.node_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn delete_map_layouts(tran:&Transaction, map_id: u32) -> Result<usize,rusqlite::Error>  {
    let sql: &str = "DELETE FROM map_layout WHERE map_id = ?1;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];   
    tran.execute(sql, params_vec)
}

pub fn save_map_data(conn:&mut Connection, model: MapData) -> Result<usize,rusqlite::Error> {
    let mut affected_row_count: usize = 0;
    let map_id: u32 = model.map_info.map_id.clone();
    let map_info = get_map_info(map_id);
    let tran: Transaction = conn.transaction().unwrap();
    // Save map_info
    if map_info.is_none() {
        match insert_map_info(&tran, model.map_info) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => {
                println!("Error: {}", e);
                tran.rollback().unwrap();
                return Err(e);
            }
        }
    }
    // Save map_node
    match delete_map_nodes(&tran, map_id) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => {
            println!("Error: {}", e);
            tran.rollback().unwrap();
            return Err(e);
        }
    }
    for map_node in model.nodes {
        match insert_map_node(&tran, map_node) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => {
                println!("Error: {}", e);
                tran.rollback().unwrap();
                return Err(e);
            }
        }
    }
    // Save map_edge
    match delete_map_edges(&tran, map_id) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => {
            println!("Error: {}", e);
            tran.rollback().unwrap();
            return Err(e);
        }
    }
    for map_edge in model.edges {   
        match insert_map_edge(&tran, map_edge) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => {
                println!("Error: {}", e);
                tran.rollback().unwrap();
                return Err(e);
            }
        }
    }
    // Save map_layout
    match delete_map_layouts(&tran, map_id) {
        Ok(row_count) => {
            affected_row_count += row_count;
        },
        Err(e) => {
            println!("Error: {}", e);
            tran.rollback().unwrap();
            return Err(e);
        }
    }
    for map_layout in model.layouts {
        match insert_map_layout(&tran, map_layout) {
            Ok(row_count) => {
                affected_row_count += row_count;
            },
            Err(e) => {
                println!("Error: {}", e);
                tran.rollback().unwrap();
                return Err(e);
            }
        }
    }
    match tran.commit() {
        Ok(_) => {
            return Ok(affected_row_count);
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e);
        }
    }
}

pub fn get_map_nodes(map_id: u32) -> Vec<MapNode> {
    let mut map_nodes: Vec<MapNode> = Vec::new();
    let conn: Connection = connect_db().unwrap();
    let sql: &str = "SELECT map_id, node_id, node_name, ip_addr, host_name FROM map_node WHERE map_id = ?1;";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];
    let mut stmt: Statement = conn.prepare(sql).unwrap();
    let mut rows: Rows = stmt.query(params_vec).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let map_node: MapNode = MapNode {
            map_id: row.get(0).unwrap(),
            node_id: row.get(1).unwrap(),
            node_name: row.get(2).unwrap(),
            ip_addr: row.get(3).unwrap(),
            host_name: row.get(4).unwrap()
        };
        map_nodes.push(map_node);
    }
    map_nodes
}

pub fn get_map_edges(map_id: u32) -> Vec<MapEdge> {
    let mut map_edges: Vec<MapEdge> = Vec::new();
    let conn: Connection = connect_db().unwrap();
    let sql: &str = "SELECT map_id, edge_id, source_node_id, target_node_id, edge_label FROM map_edge WHERE map_id = ?1";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];
    let mut stmt: Statement = conn.prepare(sql).unwrap();
    let mut rows: Rows = stmt.query(params_vec).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let map_edge: MapEdge = MapEdge {
            map_id: row.get(0).unwrap(),
            edge_id: row.get(1).unwrap(),
            source_node_id: row.get(2).unwrap(),
            target_node_id: row.get(3).unwrap(),
            edge_label: row.get(4).unwrap()
        };
        map_edges.push(map_edge);
    }
    map_edges
}

pub fn get_map_layouts(map_id: u32) -> Vec<MapLayout> {
    let mut map_layouts: Vec<MapLayout> = Vec::new();
    let conn: Connection = connect_db().unwrap();
    let sql: &str = "SELECT map_id, node_id, x_value, y_value FROM map_layout WHERE map_id = $1";
    let params_vec: &[&dyn rusqlite::ToSql] = params![
        map_id
    ];
    let mut stmt: Statement = conn.prepare(sql).unwrap();
    let mut rows: Rows = stmt.query(params_vec).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let map_layout: MapLayout = MapLayout {
            map_id: row.get(0).unwrap(),
            node_id: row.get(1).unwrap(),
            x_value: row.get(2).unwrap(),
            y_value: row.get(3).unwrap()
        };
        map_layouts.push(map_layout);
    }
    map_layouts
}

pub fn get_map_data(map_id: u32) -> MapData {
    let mut map_data: MapData = MapData::new();
    let map_info: Option<MapInfo> = get_map_info(map_id);
    if map_info.is_some() {
        map_data.map_info = map_info.unwrap();
    }else {
        return map_data;
    }
    let map_nodes: Vec<MapNode> = get_map_nodes(map_id);
    map_data.nodes = map_nodes;
    let map_edges: Vec<MapEdge> = get_map_edges(map_id);
    map_data.edges = map_edges;
    let map_layouts: Vec<MapLayout> = get_map_layouts(map_id);
    map_data.layouts = map_layouts;
    map_data
}

pub fn get_top_probe_hist() -> Vec<ProbeLog> {
    let mut results: Vec<ProbeLog> = vec![];
    let conn = connect_db().unwrap();
    let sql: &str = "SELECT A.id, A.probe_id, A.probe_type_id, B.probe_type_name, A.probe_target_addr, A.probe_target_name, A.protocol_id, A.probe_option, A.issued_at 
    FROM probe_result AS A INNER JOIN probe_type AS B ON A.probe_type_id = B.probe_type_id ORDER BY A.id DESC LIMIT 10 ; ";
    let mut stmt = conn.prepare(sql).unwrap(); 
    let result_iter = stmt.query_map([], |row| {
        Ok(ProbeLog {
            id: row.get(0).unwrap(), 
            probe_id: row.get(1).unwrap(), 
            probe_type_id: row.get(2).unwrap(), 
            probe_type_name: row.get(3).unwrap(), 
            probe_target_addr: row.get(4).unwrap(), 
            probe_target_name: row.get(5).unwrap(), 
            protocol_id: row.get(6).unwrap(), 
            probe_option: row.get(7).unwrap(), 
            issued_at: row.get(8).unwrap() 
        })
    }).unwrap();
    for result in result_iter {
        results.push(result.unwrap());
    }
    return results;
}

pub fn get_probe_stat() -> ProbeStat {
    let mut probe_stat: ProbeStat = ProbeStat::new();
    let conn = connect_db().unwrap();
    let sql: &str = "SELECT probe_type_id, COUNT(*) FROM probe_result GROUP BY probe_type_id;";
    let mut stmt = conn.prepare(sql).unwrap();
    let mut rows = stmt.query([]).unwrap();
    while let Some(row) = rows.next().unwrap() {
        let probe_type_id: String = row.get(0).unwrap();
        let count: u32 = row.get(1).unwrap();
        if probe_type_id == option::CommandType::PortScan.id() {
            probe_stat.portscan_count = count;
        }else if probe_type_id == option::CommandType::HostScan.id() {
            probe_stat.hostscan_count = count;
        }else if probe_type_id == option::CommandType::Traceroute.id() {
            probe_stat.traceroute_count = count;
        }else if probe_type_id == option::CommandType::Ping.id() {
            probe_stat.ping_count = count;
        }
    }
    probe_stat
}

pub fn save_user_probe_data(tran:&Transaction, user_data: UserProbeData) -> Result<usize, rusqlite::Error> {
    let mut affected_row_count: usize = 0;

    match user_data.host.delete(&tran) {
        Ok(count) => {
            affected_row_count += count;
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e);
        }
    }
    match user_data.host.insert(&tran) {
        Ok(count) => {
            affected_row_count += count;
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e);
        }
    }
    match UserService::delete_by_host_id(&tran, user_data.host_id.clone()){
        Ok(count) => {
            affected_row_count += count;
        },
        Err(e) => {
            println!("Error: {}", e);
            return Err(e);
        }
    }
    for service in user_data.services {
        /* match service.delete(&tran) {
            Ok(count) => {
                affected_row_count += count;
            },
            Err(e) => {
                println!("Error: {}", e);
                return Err(e);
            }
        } */
        match service.insert(&tran) {
            Ok(count) => {
                affected_row_count += count;
            },
            Err(e) => {
                println!("Error: {}", e);
                return Err(e);
            }
        }
    }
    for group_id in user_data.groups {
        let host_group: UserHostGroup = UserHostGroup {
            host_id: user_data.host_id.clone(),
            group_id: group_id.clone()
        };
        match host_group.delete(&tran) {
            Ok(count) => {
                affected_row_count += count;
            },
            Err(e) => {
                println!("Error: {}", e);
                return Err(e);
            }
        }
        match host_group.insert(&tran) {
            Ok(count) => {
                affected_row_count += count;
            },
            Err(e) => {
                println!("Error: {}", e);
                return Err(e);
            }
        }
    }
    for tag_id in user_data.tags {
        let host_tag: UserHostTag = UserHostTag {
            host_id: user_data.host_id.clone(),
            tag_id: tag_id.clone()
        };
        match host_tag.delete(&tran) {
            Ok(count) => {
                affected_row_count += count;
            },
            Err(e) => {
                println!("Error: {}", e);
                return Err(e);
            }
        }
        match host_tag.insert(&tran) {
            Ok(count) => {
                affected_row_count += count;
            },
            Err(e) => {
                println!("Error: {}", e);
                return Err(e);
            }
        }
    }
    Ok(affected_row_count)
}

pub fn get_user_probe_data() -> Vec<UserProbeData> {
    let mut user_probe_data_list: Vec<UserProbeData> = Vec::new();
    let conn = crate::db::connect_db().unwrap();
    let mut stmt = conn.prepare("SELECT host_id, ip_addr, host_name, mac_addr, vendor_name, os_name, os_cpe, valid_flag FROM user_host WHERE valid_flag = 1;").unwrap();
    let user_host_iter = stmt.query_map(params![], |row| {
        Ok(crate::db_models::UserHost {
            host_id: row.get(0)?,
            ip_addr: row.get(1)?,
            host_name: row.get(2)?,
            mac_addr: row.get(3)?,
            vendor_name: row.get(4)?,
            os_name: row.get(5)?,
            os_cpe: row.get(6)?,
            valid_flag: row.get(7)?
        })
    }).unwrap();
    for user_host in user_host_iter {
        let mut user_probe_data = UserProbeData::new();
        user_probe_data.host_id = user_host.as_ref().unwrap().host_id.clone();
        user_probe_data.host = user_host.unwrap();
        let mut stmt = conn.prepare("SELECT host_id, port, protocol, service_name, service_description, service_cpe FROM user_service WHERE host_id = ?1;").unwrap();
        let user_service_iter = stmt.query_map(params![user_probe_data.host_id.clone()], |row| {
            Ok(crate::db_models::UserService {
                host_id: row.get(0)?,
                port: row.get(1)?,
                protocol: row.get(2)?,
                service_name: row.get(3)?,
                service_description: row.get(4)?,
                service_cpe: row.get(5)?,
            })
        }).unwrap();
        for user_service in user_service_iter {
            user_probe_data.services.push(user_service.unwrap());
        }
        let mut stmt = conn.prepare("SELECT group_id FROM user_host_group WHERE host_id = ?1;").unwrap();
        let user_host_group_iter = stmt.query_map(params![user_probe_data.host_id.clone()], |row| {
            Ok(row.get(0)?)
        }).unwrap();
        for user_host_group in user_host_group_iter {
            user_probe_data.groups.push(user_host_group.unwrap());
        }
        let mut stmt = conn.prepare("SELECT tag_id FROM user_host_tag WHERE host_id = ?1;").unwrap();
        let user_host_tag_iter = stmt.query_map(params![user_probe_data.host_id.clone()], | row| {
            Ok(row.get(0)?)
        }).unwrap();
        for user_host_tag in user_host_tag_iter {
            user_probe_data.tags.push(user_host_tag.unwrap());
        }
        user_probe_data_list.push(user_probe_data);
    }
    user_probe_data_list
}

pub fn get_user_hosts() -> Vec<UserHost> {
    let mut user_hosts: Vec<UserHost> = Vec::new();
    let conn = crate::db::connect_db().unwrap();
    let mut stmt = conn.prepare("SELECT host_id, ip_addr, host_name, mac_addr, vendor_name, os_name, os_cpe, valid_flag FROM user_host;").unwrap();
    let user_host_iter = stmt.query_map(params![], |row| {
        Ok(crate::db_models::UserHost {
            host_id: row.get(0)?,
            ip_addr: row.get(1)?,
            host_name: row.get(2)?,
            mac_addr: row.get(3)?,
            vendor_name: row.get(4)?,
            os_name: row.get(5)?,
            os_cpe: row.get(6)?,
            valid_flag: row.get(7)?
        })
    }).unwrap();
    for user_host in user_host_iter {
        user_hosts.push(user_host.unwrap());
    }
    user_hosts
}

pub fn get_valid_user_hosts() -> Vec<UserHost> {
    let mut user_hosts: Vec<UserHost> = Vec::new();
    let conn = crate::db::connect_db().unwrap();
    let mut stmt = conn.prepare("SELECT host_id, ip_addr, host_name, mac_addr, vendor_name, os_name, os_cpe, valid_flag FROM user_host WHERE valid_flag = 1;").unwrap();
    let user_host_iter = stmt.query_map(params![], |row| {
        Ok(crate::db_models::UserHost {
            host_id: row.get(0)?,
            ip_addr: row.get(1)?,
            host_name: row.get(2)?,
            mac_addr: row.get(3)?,
            vendor_name: row.get(4)?,
            os_name: row.get(5)?,
            os_cpe: row.get(6)?,
            valid_flag: row.get(7)?
        })
    }).unwrap();
    for user_host in user_host_iter {
        user_hosts.push(user_host.unwrap());
    }
    user_hosts
}