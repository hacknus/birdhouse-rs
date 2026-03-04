use dioxus::prelude::*;

const SOFTWARE_SVG: Asset = asset!("/public/birdhouse.drawio.svg");
const SPECTROGRAM_IMG: Asset = asset!("/public/spectrogram.png");
const FINAL_TWO_IMG: Asset = asset!("/public/final_2.jpg");
const FINAL_IMG: Asset = asset!("/public/final.jpg");

pub fn HowItWorks() -> Element {
    rsx! {
        section { class: "min-h-screen w-full bg-white text-zinc-900 px-4 py-12",
            div { class: "mx-auto w-full max-w-6xl",
                h1 { class: "text-4xl md:text-5xl font-semibold tracking-tight mb-4", "How It Works" }
                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-4",
                    style: "text-align: justify;",
                    "The birdhouse project is based around a Nistkasten from Vogelwarte Sempbach and upgraded to capture live video and sensor data using a Raspberry Pi. It publishes the stream via mediaMTX, and serves telemetry through this website. Extended live diagnostics are available on "
                    a {
                        class: "font-medium underline decoration-zinc-400 underline-offset-2 hover:decoration-zinc-700",
                        href: "https://linusleo.synology.me/for_nerds",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "For Nerds"
                    }
                    "."
                }
                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-10",
                    style: "text-align: justify;",
                    "The first iteration was an all-in-one Django app running directly on the Raspberry Pi. The second iteration moved web delivery to a dedicated server for better reliability and scale. The original project is archived at "
                    a {
                        class: "font-medium underline decoration-zinc-400 underline-offset-2 hover:decoration-zinc-700",
                        href: "https://github.com/hacknus/birdhouse-monitor",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "github.com/hacknus/birdhouse-monitor"
                    }
                    "."
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-8 items-start mb-12",
                    figure {
                        img {
                            class: "w-full h-auto",
                            src: FINAL_TWO_IMG,
                            alt: "Inside the birdhouse with camera and sensors"
                        }
                        figcaption { class: "mt-2 text-sm text-zinc-600", "Figure 1. Camera and sensor placement inside the birdhouse." }
                    }
                    article {
                        h2 { class: "text-2xl font-semibold tracking-tight mb-3", "Components" }
                        p { class: "text-base leading-8 text-zinc-800 mb-3", "A Raspberry Pi 3B handles camera capture, GPIO sensors, and control commands. The Raspberry Pi is connected to the web-server via a TCP connection." }
                        p { class: "text-base leading-8 text-zinc-800", "The following components are mounted in the birdhouse:" }
                        ol { class: "list-disc pl-6 mt-2 space-y-1 text-base leading-8 text-zinc-800",
                            li {
                                "Camera: Raspberry Pi Camera Module v3, NoIR"
                            }
                            li {
                                "Temperature/Humidity Sensor: Sensirion SHT41"
                            }
                            li {
                                "CO2 Sensor: Sensirion SCD41"
                            }
                            li {
                                "Luminosity Sensor: TSL2561"
                            }
                            li {
                                "Radar: Acconeer A121"
                            }
                            li {
                                "USB Microphone"
                            }
                        }
                    }
                }

                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-10",
                    style: "text-align: justify;",
                    "We included a microphone to analyse the sounds of different birds. A spectrogram is shown on the website, where y-axis corresponds to the frequency (logarithmic) and the x-axis to time (full window is about 20s)."
                }
                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-10",
                    style: "text-align: justify;",
                    "If a plane flies overhead, the noise floor increases, sometimes the Doppler effect can be visible. Emergency vehicles and rain drops can also be detected."
                }

                figure { class: "mb-12",
                    img {
                        class: "w-full h-auto",
                        src: SPECTROGRAM_IMG,
                        alt: "Birdhouse mounted and operating outside"
                    }
                    figcaption { class: "mt-2 text-sm text-zinc-600", "Figure 2. Audio spectrogram showing the chirp of a male Great Tit." }
                }

                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-10",
                    style: "text-align: justify;",
                    "The camera does not have an infrared filter and is thus able to take images during the night when the birdhouse is illuminated with IR LEDs. Birds are not disturbed by infrared light, since they can see better in the UV/blue part of the spectrum than in the red/IR [1, 2]."
                }
                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-10",
                    style: "text-align: justify;",
                    "To detect motion in the birdhouse, a 60 GHz radar has been mounted on the ceiling. It is not only able to detect motion but also provides an activity score and breathing rate (experimental). 60 GHz radiation is not dangerous to birds as it does not penetrate more than micrometers into the tissue [3]."
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 gap-8 items-start mb-12",
                    article {
                        h2 { class: "text-2xl font-semibold tracking-tight mb-3", "Streaming and Web Delivery" }
                        p { class: "text-base leading-8 text-zinc-800 mb-3", "The camera stream from the Raspberry Pi is distributed through mediaMTX which exposes HLS/WebRTC stream paths." }
                        p { class: "text-base leading-8 text-zinc-800", "Live metrics and spectrogram data are rendered in parallel through Grafana and WebSocket-driven visualizations." }
                        p { class: "text-base leading-8 text-zinc-800", "The webserver and Raspberry Pi are connected through an encrypted Netbird VPN." }
                        p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-4",
                    style: "text-align: justify;",
                    "The main software for the raspberry pi is written in Python and can be found on "
                    a {
                        class: "font-medium underline decoration-zinc-400 underline-offset-2 hover:decoration-zinc-700",
                        href: "https://github.com/hacknus/birdhouse-monitor",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "https://github.com/hacknus/birdhouse-monitor"
                    }
                    ". The spectrogram streaming is written in Rust and can be found on "
                    a {
                        class: "font-medium underline decoration-zinc-400 underline-offset-2 hover:decoration-zinc-700",
                        href: "https://github.com/hacknus/spectrogram-ws-rs",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "https://github.com/hacknus/spectrogram-ws-rs"
                    }
                    ". The software for the webserver is written in Rust with Dioxus and can be found on "
                    a {
                        class: "font-medium underline decoration-zinc-400 underline-offset-2 hover:decoration-zinc-700",
                        href: "https://github.com/hacknus/birdhouse-rs",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        "https://github.com/hacknus/birdhouse-rs"
                    }
                    "."
                }
                    }
                    figure {
                        div { class: "w-full h-[420px] flex items-center justify-center overflow-hidden",
                            img {
                                class: "h-full w-auto object-contain",
                                style: "transform: rotate(90deg); transform-origin: center;",
                                src: FINAL_IMG,
                                alt: "Mounted Outside"
                            }
                        }
                        figcaption { class: "mt-2 text-sm text-zinc-600", "Figure 3. The birdhouse mounted outside of the office." }
                    }
                }

                figure { class: "mb-12",
                    img {
                        class: "w-3/4 h-auto mx-auto",
                        src: SOFTWARE_SVG,
                        alt: "Software architecture"
                    }
                    figcaption { class: "mt-2 text-sm text-zinc-600", "Figure 4. Software architecture of the birdhouse project." }
                }

                section { class: "border-t border-zinc-200 pt-4",
                    h3 { class: "text-base font-semibold mb-2", "References" }
                    ol { class: "list-decimal pl-5 space-y-2 text-sm text-zinc-700",
                        li {
                            "Lind, O., Mitkus, M., Olsson, P., Kelber, A.; Ultraviolet vision in birds: the importance of transparent eye media. Proc Biol Sci 1 January 2014; 281 (1774): 20132209. "
                            a {
                                class: "underline underline-offset-2",
                                href: "https://doi.org/10.1098/rspb.2013.2209",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                "https://doi.org/10.1098/rspb.2013.2209"
                            }
                        }
                        li {
                            "Chen, D. M., Goldsmith, T. H. Four spectral classes of cone in the retinas of birds. J Comp Physiol A. 1986 Oct;159(4):473-9. "
                            a {
                                class: "underline underline-offset-2",
                                href: "https://doi.org/10.1007/BF00604167",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                "https://doi.org/10.1007/BF00604167"
                            }
                        }
                        li {
                            "Adekola, S.A., Amusa, K.A. & Biowei, G. Impact of 5G mmWave radiation on human tissue using skin, cornea (eye) and enamel (tooth) as study candidates. J. Eng. Appl. Sci. 72, 51 (2025). "
                            a {
                                class: "underline underline-offset-2",
                                href: "https://doi.org/10.1186/s44147-025-00617-9",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                "https://doi.org/10.1186/s44147-025-00617-9"
                            }
                        }
                    }
                }
            }
        }
    }
}
