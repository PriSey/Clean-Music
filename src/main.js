const { invoke } = window.__TAURI__.core;
const {listen} = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

let greetInputEl;
let greetMsgEl;

let folderSelectButton;
let lbd;
let song_disp;
let pos_interaction = false;

listen("LeftbarDirButtons", (event) => {

  let directory_button = document.createElement("button");
  directory_button.dataset.path = event.payload.id;
  directory_button.id = event.payload.id + "LeftDirBar";
  directory_button.innerText = event.payload.text;
  directory_button.dataset.name = event.payload.text;
  directory_button.classList.add(event.payload.class);

  directory_button.addEventListener("click", function(clickEvent){
    directoryButtonFunction(clickEvent);
  });

  lbd.appendChild(directory_button);

}); 

listen("CreateSongButtons", (event) => {

  let song_box = document.createElement("div");
  song_box.classList.add("songBox")
  let title = document.createElement("span");
  title.innerText = event.payload.text;
  title.classList.add("songTitle")
  let play = document.createElement("button");
  play.classList.add("PButton")
  play.classList.add("PQButton")
  play.dataset.path = event.payload.id;
  let play_img = document.createElement("img")
  play_img.src = "./assets/play.svg"
  play_img.classList.add("icon");
  play.append(play_img);

  play.addEventListener("click", async function(clickEvent) {
    let inner_path = clickEvent.target.dataset.path;
    await invoke("play_song",{path:inner_path});
  });

  let queue = document.createElement("button");
  queue.classList.add("QButton")
  queue.classList.add("PQButton")
  queue.dataset.path = event.payload.id;
  let queue_img = document.createElement("img")
  queue_img.src = "./assets/queue.svg"
  queue_img.classList.add("icon");
  queue.append(queue_img);

  queue.addEventListener("click", async function(clickEvent) {
    let inner_path = clickEvent.target.dataset.path;
    await invoke("queue_song",{path:inner_path});
  });


  let button_box = document.createElement("div");
  button_box.classList.add("ButtonBox");

  song_box.appendChild(title);
  button_box.appendChild(play);
  button_box.appendChild(queue);

  song_box.appendChild(button_box)

  song_disp.appendChild(song_box);

});

async function directoryButtonFunction(clickEvent){
  let folder_disp = document.getElementById("folderDisp");

  song_disp.replaceChildren();
  folder_disp.replaceChildren();

  let target_id = event.target.id;
  let target_path = event.target.dataset.path;

  let folder_name = document.createElement("span");
  folder_name.textContent = document.getElementById(target_id).dataset.name;
  folder_name.classList.add("left_disp");

  let play = document.createElement("button");
  play.dataset.path = target_path;
  play.classList.add("PButton")
  play.classList.add("PQButton")
  let play_img = document.createElement("img")
  play_img.src = "./assets/play.svg"
  play_img.classList.add("icon");
  play.append(play_img);


  let queue = document.createElement("button");
  let queue_img = document.createElement("img")
  queue_img.src = "./assets/queue.svg"
  queue_img.classList.add("icon");
  queue.append(queue_img);

  queue.dataset.path = target_path;
  queue.classList.add("QButton");
  queue.classList.add("PQButton")


  let isDir = await invoke("check_dir",{path: target_path});


  if (isDir){
    play.addEventListener("click", async function(clickEvent) {
    let inner_path = clickEvent.target.dataset.path;
    await invoke("play_dir",{path:inner_path});
    });

    queue.addEventListener("click", async function(clickEvent) {
      let inner_path = clickEvent.target.dataset.path;
      await invoke("queue_dir",{path:inner_path});
    });

    await invoke("fetch_songs",{path: target_path})
  }

  else{
    play.addEventListener("click", async function(clickEvent) {
    let inner_path = clickEvent.target.dataset.path;
    await invoke("play_song",{path:inner_path});
    });

    queue.addEventListener("click", async function(clickEvent) {
      let inner_path = clickEvent.target.dataset.path;
      await invoke("queue_song",{path:inner_path});
    });
  }


  let button_box = document.createElement("div");
  button_box.classList.add("ButtonBox");

  folder_disp.appendChild(folder_name);

  button_box.appendChild(play);
  button_box.appendChild(queue);

  folder_disp.appendChild(button_box);



}


async function pickDirectory(btn){
  lbd = document.getElementById("left-bar-directories");
  const file = await open({
  multiple: false,
  directory: true,
  });
  lbd.replaceChildren();
  let folder_name = file.split("/")
  folderSelectButton.querySelector("#path-slector-text").innerText = folder_name[folder_name.length - 1];
  await invoke("load_from_music_folder", {path: file})
}

async function updateCurrentPlaying(){
  let is_playing = await invoke("get_up");
  let current_playing_disp = document.getElementById("current-playing");
  let song_title_disp = current_playing_disp.querySelector("#song-title");
  if (is_playing){
    let song_title = await invoke("get_current_song");
    song_title_disp.innerText = song_title;
  }
  else{
    song_title_disp.innerText = ""
  }

}

document.addEventListener("click", async function(event){
  if(event.target.classList.contains("actionButton")){
      let cmd = Number(event.target.id.at(-1));
      if (event.target.classList.contains("changer")){
        let intended = Number(event.target.id.at(-2));
        let new_id_number = intended;
        let inner_img = event.target.querySelector(".ActionButtonImg");
        if (intended == cmd){
          new_id_number += 1;
          inner_img.src = inner_img.dataset.image2;
        } else {
          inner_img.src = inner_img.dataset.image1;
        }
        event.target.id = "a" + intended + new_id_number;
      }
      await invoke("send_command", {cmd: cmd});
  }
});

async function updatePosition(range){
  let is_playing = await invoke("get_up");
  if (is_playing && !pos_interaction){
    range.disabled = false;
    range.value = await invoke("get_position");
  } else if (is_playing && pos_interaction){
      range.disabled = false;
  } else {
    range.value = 0;
    range.disabled = true;
  }
}

document.getElementById("position").addEventListener("pointerdown", function() {
  pos_interaction = true;
});

document.getElementById("position").addEventListener("pointerup", function() {
  pos_interaction = false;
});

document.getElementById("position").addEventListener("change", async function(e) {
  let position = e.target.valueAsNumber;
  await invoke("seek_position", {virtualPosition: position});
});


window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");

  folderSelectButton = document.getElementById("folder-selector");
  song_disp = document.getElementById("songDisp");
  folderSelectButton.addEventListener("click", function() {
    pickDirectory(folderSelectButton);
  });

  let volume_slider = document.getElementById("volume");
  volume_slider.addEventListener("input", async function(event){
    let volume = volume_slider.value;
    await invoke("set_volume", {volume: volume});
  });

  setInterval(function() {
      updatePosition(document.getElementById("position"));
      updateCurrentPlaying();
  },100);

});

