use dioxus::prelude::*;

const BIRD_VIDEO_ONE: Asset = asset!("/public/birdie_anflug.mp4");
const BIRD_VIDEO_TWO: Asset = asset!("/public/birdie_anflug2.mp4");
const BIRD_VIDEO_THREE: Asset = asset!("/public/birdie_anflug3.mp4");
const BIRD_VIDEO_FOUR: Asset = asset!("/public/birdie_takeoff.mp4");
const BIRD_POSTER_ONE: Asset = asset!("/public/birdie_anflug.jpg");
const BIRD_POSTER_TWO: Asset = asset!("/public/birdie_anflug2.jpg");
const BIRD_POSTER_THREE: Asset = asset!("/public/birdie_anflug3.jpg");
const BIRD_POSTER_FOUR: Asset = asset!("/public/birdie_takeoff.jpg");

pub fn Birds() -> Element {
    rsx! {
        section { class: "min-h-screen w-full bg-white text-zinc-900 px-4 py-12",
            div { class: "mx-auto w-full max-w-6xl",
                h1 { class: "text-4xl md:text-5xl font-semibold tracking-tight mb-4", "Birds" }
                p { class: "w-full text-base md:text-lg leading-8 text-zinc-700 mb-10",
                    style: "text-align: justify;",
                    "A small collection of bird clips recorded in and around the birdhouse."
                }

                // div { class: "grid grid-cols-1 md:grid-cols-2 gap-8 items-start mb-12",
                //     article {
                //         h2 { class: "text-2xl font-semibold tracking-tight mb-3", "Recorded Visitors" }
                //         p { class: "text-base leading-8 text-zinc-800 mb-3",
                //             "Each clip can be streamed inline in the browser. The page expects the files to be available under the server's "
                //             code { "/public/" }
                //             " directory."
                //         }
                //         p { class: "text-base leading-8 text-zinc-800",
                //             "Current filenames used by the page are "
                //             code { "birdie_anflug.mp4" }
                //             ", "
                //             code { "birdie_anflug2.mp4" }
                //             ", and "
                //             code { "birdie_anflug3.mp4" }
                //             "."
                //         }
                //     }
                //     article {
                //         h2 { class: "text-2xl font-semibold tracking-tight mb-3", "Playback" }
                //         p { class: "text-base leading-8 text-zinc-800 mb-3",
                //             "Videos use native browser controls and are muted by default so the page can load cleanly on desktop and mobile."
                //         }
                //         p { class: "text-base leading-8 text-zinc-800",
                //             "If a file is missing on the server, the browser video player will simply fail to load that clip."
                //         }
                //     }
                // }

                div { class: "space-y-12",
                    figure {
                        h2 { class: "text-2xl font-semibold tracking-tight mb-4", "Female Approach 1" }
                        div { class: "w-full rounded-2xl overflow-hidden",
                            video {
                                class: "block w-full border-0 outline-none bg-transparent",
                                style: "border: 0; outline: none;",
                                controls: true,
                                preload: "metadata",
                                muted: true,
                                playsinline: true,
                                poster: BIRD_POSTER_ONE,
                                source { src: BIRD_VIDEO_ONE, r#type: "video/mp4" }
                            }
                        }
                        figcaption { class: "mt-2 text-sm text-zinc-600", "Slow motion footage of the female great tit on approach." }
                    }

                    figure {
                        h2 { class: "text-2xl font-semibold tracking-tight mb-4", "Female Approach 2" }
                        div { class: "w-full rounded-2xl overflow-hidden",
                            video {
                                class: "block w-full border-0 outline-none bg-transparent",
                                style: "border: 0; outline: none;",
                                controls: true,
                                preload: "metadata",
                                muted: true,
                                playsinline: true,
                                poster: BIRD_POSTER_TWO,
                                source { src: BIRD_VIDEO_TWO, r#type: "video/mp4" }
                            }
                        }
                        figcaption { class: "mt-2 text-sm text-zinc-600", "Slow motion footage of the female great tit on approach with material for the nest construction." }
                    }

                    figure {
                        h2 { class: "text-2xl font-semibold tracking-tight mb-4", "Female Approach 3" }
                        div { class: "w-full rounded-2xl overflow-hidden",
                            video {
                                class: "block w-full border-0 outline-none bg-transparent",
                                style: "border: 0; outline: none;",
                                controls: true,
                                preload: "metadata",
                                muted: true,
                                playsinline: true,
                                poster: BIRD_POSTER_THREE,
                                source { src: BIRD_VIDEO_THREE, r#type: "video/mp4" }
                            }
                        }
                        figcaption { class: "mt-2 text-sm text-zinc-600", "Slow motion footage of the female great tit on approach with material for the nest construction." }
                    }

                    figure {
                        h2 { class: "text-2xl font-semibold tracking-tight mb-4", "Male Departure" }
                        div { class: "w-full rounded-2xl overflow-hidden",
                            video {
                                class: "block w-full border-0 outline-none bg-transparent",
                                style: "border: 0; outline: none;",
                                controls: true,
                                preload: "metadata",
                                muted: true,
                                playsinline: true,
                                poster: BIRD_POSTER_FOUR,
                                source { src: BIRD_VIDEO_FOUR, r#type: "video/mp4" }
                            }
                        }
                        figcaption { class: "mt-2 text-sm text-zinc-600", "The male great tit taking off." }
                    }
                }
            }
        }
    }
}
