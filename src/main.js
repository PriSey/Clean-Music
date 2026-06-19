const { invoke } = window.__TAURI__.core;
const {listen} = window.__TAURI__.event;
const { open } = window.__TAURI__.dialog;

let greetInputEl;
let greetMsgEl;

let folderSelectButton;
let lbd;
let song_disp;

listen("LeftbarDirButtons", (event) => {
  console.log("Displaying Directories")
  console.log(event.payload)

  console.log(lbd);2

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
  console.log(event.payload);

  let song_box = document.createElement("div");
  song_box.classList.add("songBox")
  let title = document.createElement("span");
  title.innerText = event.payload.text;
  title.classList.add("songTitle")
  let play = document.createElement("button");
  play.classList.add("songPlayButton")
  play.dataset.path = event.payload.id;
  play.innerText = "play";

  play.addEventListener("click", async function(clickEvent) {
    let inner_path = clickEvent.target.dataset.path;
    await invoke("play_song",{path:inner_path});
  });

  let queue = document.createElement("button");
  queue.classList.add("songQueueButton")
  queue.dataset.path = event.payload.id;
  queue.innerText = "queue"

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
  play.innerText = "play";
  play.dataset.path = target_path;
  play.classList.add("dispPlay")

  let queue = document.createElement("button");
  queue.innerText = "queue";
  queue.dataset.path = target_path;
  queue.classList.add("dispQueue");

  let isDir = await invoke("check_dir",{path: target_path});

  console.log(isDir)

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

document.addEventListener("click", async function(event){
  if(event.target.classList.contains("actionButton")){
      await invoke("send_command", {cmd: event.target.id});
  }
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
  volume_slider.addEventListener("change", async function(event){
    let volume = volume_slider.value;
    console.log(volume);
    await invoke("set_volume", {volume: volume});
  });

});

