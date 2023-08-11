use std::collections::{HashMap, hash_map};
use std::fs;
use std::hash::Hash;
use std::path::Path;
use std::io::{self, BufRead,Write};
// use std::process::exit;
use metaflac::block::PictureType::Media;
use id3::{Error, ErrorKind, TagLike, Version,Frame};
use id3::frame::{Content, Lyrics, Picture, PictureType};
use indexmap::IndexMap;
use std::path::PathBuf;
use reqwest::header;
const COOKIE_PATH: &str="C:/Users/jinchuan/Music/test/cookie.txt";
#[derive(Debug)]
struct Music{
    id: String,
    name: String,
    pic_url: String,
    singer: String,
    album: String,
    donw_url: String,
    file_type: String,


}
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<fs::File>>>
where P: AsRef<Path>, {
    let file = fs::File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
fn sy_re(s:String) -> String {
    let v2:Vec<[&str;2]>=vec![["<","＜"],[">","＞"],["\\","＼"],["/","／"],[":","："],["?","？"],["*","＊"],["\"","＂"],["|","｜"],["..."," "]];
    v2.iter()
    .fold(s, |acc, &[from, to]| acc.replace(from, to))
}
fn merged_lyric(lyric:String,tlyric:String) -> String {
    let mut lyric_map: IndexMap<String, String> = IndexMap::new();
    
    for line in lyric.lines() {
        let mut parts = line.splitn(2, "]");
        let time = &parts.next().unwrap()[1..];
        let text = parts.next().unwrap().to_string();
        
        lyric_map.insert(time.to_owned(), text);
        // lyric_
    }
    
    let mut tlyric_map: IndexMap<String, String> = IndexMap::new();
    for line in tlyric.lines() {
        let mut parts = line.splitn(2, "]");
        let time = &parts.next().unwrap().to_string()[1..];
        let text = parts.next().unwrap().to_string();
        
        tlyric_map.insert(time.to_owned(), text); 
    }
    let mut merged: String = String::new();
    
    for (time,c) in lyric_map {
        if tlyric_map.get(&time).is_none(){
            merged.push_str(&format!("[{}]{}\n",&time,c.as_str()));
        }else{
            merged.push_str(&format!("[{}]{}\n[{}]{}\n", &time , c.as_str() ,  time, tlyric_map.get(&time).unwrap()));

        }


    }
    merged

}
async fn init(p:&str,_id:&str){
    let path: PathBuf =PathBuf::from(p);
    match get_song_list(&path,_id).await{
        Ok(_o)=>{

        }
        Err(_e)=>{
            println!("或许未开启服务");
        }
    }

}
async fn get_song_list(path:&PathBuf,_id: &str)-> Result<(), reqwest::Error>{
    let resp=reqwest::get("http://localhost:3000/playlist/detail?id=6904724287").await?.json::<serde_json::Value>().await?;

    println!("{:#?}", resp["playlist"]["trackCount"]);



    let mut all_music: Vec<Music>=Vec::new();
    //收集
    for i in 0..=resp["playlist"]["trackCount"].as_i64().unwrap()/50{
        let resp=reqwest::get(format!("http://localhost:3000/playlist/track/all?id={}&limit=50&offset={}",_id,i*50)).await?.json::<serde_json::Value>().await?;
        // let all_id=resp["songs"]
        for _i in resp["songs"].as_array().unwrap(){

            let ar=_i["ar"].as_array().unwrap();
            let ars= if ar.len()>=3 {
                ar.get(..3)
            } else {
                ar.get(..)
            }.unwrap()
            .iter()
            .map(|obj| obj["name"].as_str().unwrap())
            .collect::<Vec<_>>();

            let music=Music{
                id: _i["id"].as_i64().unwrap().to_string(),
                name: format!("{} - {}",  ars.join(","),  sy_re(_i["name"].as_str().unwrap().to_string())),
                pic_url: _i["al"]["picUrl"].as_str().unwrap().to_string(),
                singer: sy_re(ars.join(",")),
                album: _i["al"]["name"].as_str().unwrap().to_string(),
                
                donw_url: String::new(),
                file_type: String::new(),
            };
            all_music.push(music);
        }
    }
    let mut com_music=Vec::new();
    let _tmp_path=path.clone();
    let mu_id: String=_tmp_path.to_str().unwrap().to_string()+"music_id.txt";
    //补集
    match fs::metadata(path){
        Ok(_)=>{
            if let Ok(lines) = read_lines(&mu_id) {
                let mut sub_id=Vec::new();
                for line in lines{
                    if let Ok(p) = line{

                        // 前前后后使用HashSet和BTreeSet 不能和Vec保持同一顺序
                        //因为排序问题，还是用Vec
                        sub_id.push(p.trim().split("----").collect::<Vec<_>>()[0].to_string());
                    }
                }
                com_music = all_music.into_iter().filter(|music| !sub_id.contains(&music.id)).collect();
            }
        },
        Err(_)=>{
            com_music=all_music;
        },

    };
    if!com_music.is_empty(){
        println!("yes");
        let music:HashMap<String,Music>=com_music.drain(..).map(|x| (x.id.to_owned(),x)).collect();
        let _=down_song(music,path).await;
    }

    Ok(())

}

async fn down_song(music:HashMap<String,Music>,path:&PathBuf) ->Result<(),reqwest::Error>{
    let client: reqwest::Client = reqwest::Client::new();
    let mut new_music=music;
    //遍历出所有id
    let _ids :Vec<_>=new_music.keys().map(|x|x.to_owned()).collect();
    let resp=client.get(format!("http://localhost:3000/song/url?id={}",_ids.join(","))).header(header::COOKIE, fs::read_to_string(COOKIE_PATH).unwrap().as_str()).send().await?.json::<serde_json::Value>().await?;
    let tmp_path=path.clone().to_str().unwrap().to_string();
    //获取data中的所有id和下载url
    let mut data=HashMap::new();
    for i in resp["data"].as_array().unwrap(){
        data.insert(i["id"].as_i64().unwrap().to_string(), i["url"].as_str().unwrap().to_string());
    }

    for (key, value) in &data {
        if let Some(music_entry) = new_music.get_mut(key) {
            music_entry.donw_url = value.clone();
            music_entry.file_type=value.clone().split(".").last().unwrap().to_ascii_lowercase();
        }
    }

    for (_id,_music) in  new_music.iter(){
        let filename=format!("{}.{}",_music.name,_music.file_type);
        let filepath=tmp_path.clone() + &filename;
        //音乐数据
        let music_data=reqwest::get(_music.donw_url.to_string()).await?.bytes().await?;
        //写入
        let mut file=fs::File::create(&filepath).unwrap();
        let _=file.write_all(&music_data);
        //编辑标签
        let _=edit_tag(&filepath ,_music).await;

        println!("{:#?} 下载完成",_music);

    }
    Ok(())
}
async fn edit_tag(filename:&str,music:&Music) ->Result<(), Box<dyn std::error::Error>>{

    println!("{:#?}",music);
    if music.file_type.eq("mp3"){
        // println!("mp3");

        let mut tag = match id3::Tag::read_from_path(&filename) {
            Ok(tag) => tag,
            Err(Error{kind: ErrorKind::NoTag, ..}) => id3::Tag::new(),
            Err(err) => return Err(Box::new(err)),
        };

        let resp=reqwest::get(format!("http://localhost:3000/lyric/?id={}",music.id)).await?.json::<serde_json::Value>().await?;
        let lyric=resp["lrc"]["lyric"].as_str().unwrap().to_string();
        let _tlyric=resp["tlyric"]["lyric"].as_str().unwrap();

        let lyric=merged_lyric(lyric, _tlyric.to_string());


        let data=reqwest::get(music.pic_url.to_string()).await?.bytes().await?.to_vec();

        // encoding=3, mime="image/jpeg", type=6, desc=u"Cover", data=pic_datav
        let picture=Picture{
            mime_type: String::from("image/jpeg"),
            picture_type: PictureType::Media,
            description: String::from("Cover"),
            data: data
        };

        let l=Lyrics{
          lang: String::from("chi"),
          description: String::new(),
          text: lyric,
        };
    
        //歌词
        tag.add_frame(Frame::with_content("USLT",Content::Lyrics(l.clone())));
        //image
        tag.add_frame(Frame::with_content("APIC", Content::Picture(picture.clone())));
        //title
        tag.set_album(music.album.to_string());
        //artist
        tag.set_artist(music.singer.to_string());
        //save
        tag.write_to_path(&filename, Version::Id3v23)?;
        Ok(())
    }else{
        let mut tag = metaflac::Tag::read_from_path(&filename).unwrap();
    
        let data=reqwest::get(music.pic_url.to_string()).await?.bytes().await?.to_vec();
    
        tag.add_picture("image/jpeg", Media, data);
        
    
        let comment=tag.vorbis_comments_mut();
    
        let resp=reqwest::get(format!("http://localhost:3000/lyric/?id={}",music.id)).await?.json::<serde_json::Value>().await?;
        let lyric=resp["lrc"]["lyric"].as_str().unwrap().to_string();
        let _tlyric=resp["tlyric"]["lyric"].as_str().unwrap();
    
        //可选，合并译文
        let lyric=merged_lyric(lyric, _tlyric.to_string());
    
        comment.set_lyrics(vec![lyric]);
        comment.set_album(vec![music.album.to_string()]);
        comment.set_artist(vec![music.singer.to_string()]);
    
        tag.save().unwrap();
        Ok(())
    }

}
#[tokio::main]

async fn main() {
    init("C:/Users/jinchuan/Music/CloudMusic1/","6904724287").await;

}

// async fn test() ->Result<(),reqwest::Error>{
//     let id=["1501529238","1459281279","1437240162","33469247","1437176263","1986436510","825325","1343016814","1973608593","27571001"];
//     for i in id{
//         let resp=reqwest::get(format!("http://localhost:3000/lyric/?id={}",i)).await?.json::<serde_json::Value>().await?;
//         let lyric=resp["lrc"]["lyric"].as_str().unwrap().to_string();
//         let _tlyric=resp["tlyric"]["lyric"].as_str().unwrap();
//         let lyric=merged_lyric(lyric, _tlyric.to_string());
//         println!("{}",lyric);
//     }
//     Ok(())

// }


// // use std::fs::copy;

// fn test1() -> Result<(), Box<dyn std::error::Error>> {
//     // copy("testdata/quiet.mp3", "/tmp/music.mp3")?;

//     let mut tag = match id3::Tag::read_from_path("/tmp/music.mp3") {
//         Ok(tag) => tag,
//         Err(id3::Error{kind: ErrorKind::NoTag, ..}) => id3::Tag::new(),
//         Err(err) => return Err(Box::new(err)),
//     };

//     tag.set_album("Fancy Album Title");

//     tag.write_to_path("/tmp/music.mp3",Version::Id3v24)?;
//     Ok(())
// }