export const download = {
  filename: "breaktime.exe",
  href: "https://github.com/MovieHolic-Plex/nomad-timer/releases/download/v0.1.0/breaktime.exe",
  sha256: "614e25c75c875f7f4be6b5b8da22fefe71a5e205154202cc5974b440b25db7d8"
} as const;

export const apiBaseUrl = "https://nomad-timer.hyeon.space/api" as const;

export const catScenes = [
  {
    id: "work",
    title: "집중 고양이",
    caption: "일하는 시간에는 노트북 앞에 앉습니다.",
    image: "/assets/cats/cat-work.png",
    tone: "work"
  },
  {
    id: "stretch",
    title: "기지개 고양이",
    caption: "쉬는시간이 오면 몸을 쭉 풉니다.",
    image: "/assets/cats/cat-stretch.png",
    tone: "rest"
  },
  {
    id: "sleep",
    title: "졸림 고양이",
    caption: "너무 오래 달렸을 땐 조용히 눕습니다.",
    image: "/assets/cats/cat-sleep.png",
    tone: "soft"
  },
  {
    id: "cheer",
    title: "응원 고양이",
    caption: "프리셋 반응으로 작게 파이팅을 던집니다.",
    image: "/assets/cats/cat-cheer.png",
    tone: "spark"
  },
  {
    id: "water",
    title: "물컵 고양이",
    caption: "휴식에는 물 한 잔도 같이 챙깁니다.",
    image: "/assets/cats/cat-water.png",
    tone: "rest"
  },
  {
    id: "wave",
    title: "인사 고양이",
    caption: "같은 리듬에 들어온 사람에게 손을 흔듭니다.",
    image: "/assets/cats/cat-wave.png",
    tone: "hello"
  }
] as const;

export const rhythmCards = [
  {
    label: "00-49",
    title: "몰입",
    body: "정시부터 49분까지는 모두 같은 일하는 시간입니다."
  },
  {
    label: "50-59",
    title: "휴식",
    body: "50분이 되면 전 세계의 Nomad Timer가 같은 표정으로 바뀝니다."
  },
  {
    label: "preset",
    title: "반응",
    body: "준비된 말과 이모트만 보내서 가볍고 안전한 broadcast 느낌을 만듭니다."
  }
] as const;

export const apiRoutes = [
  ["Base", apiBaseUrl],
  ["Schedule", "GET /schedule"],
  ["Broadcast", "POST /broadcast"],
  ["Messages", "GET /messages"]
] as const;
