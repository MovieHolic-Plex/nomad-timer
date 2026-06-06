import { apiRoutes, catScenes, download, rhythmCards } from "./content";
import "./styles.css";
import "./sections.css";

const quickPresets = ["휴식", "기지개", "물", "인사", "응원", "복귀"] as const;

export function App(): React.ReactElement {
  return (
    <main>
      <section className="hero" aria-labelledby="hero-title">
        <nav className="nav" aria-label="주요">
          <a className="brand" href="/">
            <span className="brand__mark" aria-hidden="true">
              50
            </span>
            <span>Nomad Timer</span>
          </a>
          <div className="nav__links">
            <a href="#cats">고양이</a>
            <a href="#api">API</a>
            <a className="nav__download" href={download.href} download>
              Windows
            </a>
          </div>
        </nav>

        <div className="hero__content">
          <div className="hero__copy">
            <p className="eyebrow">Windows native break widget</p>
            <h1 id="hero-title">쉬는시간을 같이 맞추는 작은 고양이</h1>
            <p className="hero__lead">
              정시 기준 50분은 몰입, 10분은 휴식.
              <br />
              오른쪽 아래 위젯에서
              <br />
              프리셋 반응을 주고받습니다.
            </p>
            <div className="hero__actions">
              <a className="button button--primary" href={download.href} download>
                Windows 앱 다운로드
              </a>
              <a className="button button--quiet" href="#rhythm">
                작동 방식
              </a>
            </div>
            <p className="checksum">SHA-256 {download.sha256}</p>
          </div>

          <div className="widget-demo" aria-label="Nomad Timer Windows 위젯 미리보기">
            <div className="desktop-strip" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>
            <article className="widget-card">
              <div className="widget-card__top">
                <img src="/assets/cats/cat-work.png" alt="Windows 위젯 속 집중 고양이" />
                <div>
                  <span>일하는 시간</span>
                  <strong>24:18</strong>
                </div>
              </div>
              <p>final-qa: 물 마시기</p>
              <div className="preset-row" aria-label="프리셋 반응 예시">
                {quickPresets.map((preset) => (
                  <span key={preset}>{preset}</span>
                ))}
              </div>
            </article>
          </div>
        </div>
      </section>

      <section id="rhythm" className="rhythm" aria-labelledby="rhythm-title">
        <div className="section-heading">
          <p className="section-label">Shared Rhythm</p>
          <h2 id="rhythm-title">서버 기준, 조용한 화면.</h2>
        </div>
        <div className="rhythm__grid">
          {rhythmCards.map((card) => (
            <article key={card.label}>
              <span className="step">{card.label}</span>
              <h3>{card.title}</h3>
              <p>{card.body}</p>
            </article>
          ))}
        </div>
      </section>

      <section id="cats" className="cats" aria-labelledby="cats-title">
        <div className="section-heading">
          <p className="section-label">Pixel Cats</p>
          <h2 id="cats-title">상황별 고양이를 앱 안에 넣었습니다.</h2>
        </div>
        <div className="cats__grid">
          {catScenes.map((scene) => (
            <article className={`scene scene--${scene.tone}`} key={scene.id}>
              <img src={scene.image} alt={scene.title} />
              <div>
                <h3>{scene.title}</h3>
                <p>{scene.caption}</p>
              </div>
            </article>
          ))}
        </div>
      </section>

      <section className="download" aria-labelledby="download-title">
        <div>
          <p className="section-label">Download</p>
          <h2 id="download-title">exe 하나로 실행됩니다.</h2>
          <p>Electron 없이 Rust Win32로 만들었고, 고양이 이미지는 앱 안에 내장했습니다.</p>
        </div>
        <a className="button button--primary" href={download.href} download>
          {download.filename}
        </a>
      </section>

      <section id="api" className="api" aria-labelledby="api-title">
        <div className="section-heading">
          <p className="section-label">API</p>
          <h2 id="api-title">Public API</h2>
        </div>
        <dl>
          {apiRoutes.map(([label, value]) => (
            <div key={label}>
              <dt>{label}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
      </section>
    </main>
  );
}
