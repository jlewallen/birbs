import { dev, browser } from '$app/environment';
import _ from "lodash";

$: if (browser) document.body.classList.toggle('green', false);

export async function load({ params }) {
  console.log("bird-slug", params.slug);

  // http://192.168.0.164/By_Date/2023-04-11/Trumpeter_Swan/Trumpeter_Swan-85-2023-04-11-birdnet-20:15:10.mp3

  const files = fetch(`http://127.0.0.1:3100/${params.slug}/files.json`)
    .then((res) => res.json())
    .then((rows) => {
      return _(rows)
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
        .value();
    });
  return {
    bird: {
      common_name: params.slug,
      photo_url: `http://127.0.0.1:3100/${params.slug}/photo.png`,
    },
    files: files,
  };
}
