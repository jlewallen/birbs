import { dev, browser } from '$app/environment';
import _ from "lodash";

$: if (browser) document.body.classList.toggle('green', false);

export async function load({ fetch, params }) {
  console.log("bird-slug", params.slug);

  const daily = fetch(`http://127.0.0.1:3100/${params.slug}/daily.json`)
    .then((res) => res.json());

  const hourly = fetch(`http://127.0.0.1:3100/${params.slug}/hourly.json`)
    .then((res) => res.json());

  const files = fetch(`http://127.0.0.1:3100/${params.slug}/files.json`)
    .then((res) => res.json())
    .then((resp) => {
      return {
        detections: resp.detections,
        files: _(resp.files)
        .map((row): { when: Date;
          confidence: number;
          file_name: string;
          spectrogram_url: string;
          audio_url: string;
          available: boolean;
        } => {
          return {
            when: new Date(row.when),
            confidence: row.confidence,
            file_name: row.file_name,
            spectrogram_url: row.spectrogram_url,
            audio_url: row.audio_url,
            available: row.available,
          };
        })
        .orderBy((row) => row.when)
        .reverse()
        .value()
      };
    });

  return {
    bird: {
      common_name: params.slug,
      photo_url: `http://127.0.0.1:3100/${params.slug}/photo.png`,
    },
    files,
    hourly,
    daily,
  };
}
