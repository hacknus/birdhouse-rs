use dioxus::prelude::*;

pub fn MakingOf() -> Element {
    rsx! {
        style {
            r#"
            .making-of-section {{
                min-height: 1200px;
                background-color: #0b0b0f;
                padding: 4rem 2rem;
            }}

            .making-of-title {{
                font-size: 3rem;
                font-weight: 600;
                text-align: center;
                margin-bottom: 4rem;
                color: #f5f5f7;
                letter-spacing: -0.02em;
            }}

            .making-of-subtitle {{
                gap: 3rem 2rem;
                max-width: 1200px;
                margin: 0 auto;
                text-align: center;
                margin-bottom: 4rem;
                color: #f5f5f7;
            }}

            .making-of-grid {{
                display: grid;
                grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
                gap: 3rem 2rem;
                max-width: 1200px;
                margin: 0 auto;
            }}

            .making-of-item {{
                display: flex;
                flex-direction: column;
                gap: 1rem;
            }}

            .making-of-image-wrap {{
              width: 100%;
              aspect-ratio: 4 / 3;   /* pick what you want your cards to be */
              overflow: hidden;      /* hide corners when rotated/cropped */
            }}

            .making-of-image {{
              width: 100%;
              height: 100%;
              object-fit: cover;     /* use contain if you don’t want cropping */
              display: block;
              transform-origin: center;
            }}

            /* pick ONE of these depending on what fixes it */
            .rotate-fix {{ transform: rotate(90deg); }}

            .making-of-caption {{
                font-size: 17px;
                font-weight: 500;
                color: #f5f5f7;
                letter-spacing: -0.01em;
            }}

            .making-of-description {{
                font-size: 15px;
                font-weight: 300;
                color: rgba(255, 255, 255, 0.70);
                letter-spacing: -0.016em;
                line-height: 1.4;
            }}

            @media (max-width: 768px) {{
                .making-of-grid {{
                    grid-template-columns: 1fr;
                    gap: 3rem;
                }}
            }}



            "#
        }

        div { class: "making-of-section",
            h1 { class: "making-of-title", "making of" }
            p { class: "making-of-subtitle",
                "A behind-the-scenes look at the creation of Vögeli. Since we noticed a lot of birds building their nests inside the housing of the shutters for our office windows, we thought we would approach this in a scientific way: conduct some behavior studies. We thus built a comfortable bird hotel with Orwell-esque surveillance."
            }

            div { class: "making-of-grid",
                // Step 1
                div { class: "making-of-item",
                    img { class: "making-of-image", src: "/inside.jpg", alt: "Hardware Setup" }
                    div { class: "making-of-caption", "Camera, motion sensor and temperature/humidity sensor inside of the birdhouse." }
                    div { class: "making-of-description", "We started by assembling the Raspberry Pi, camera, and motion sensors inside the birdhouse." }
                }

                // Step 2
                div { class: "making-of-item",
                    img { class: "making-of-image", src: "/electronics.jpg", alt: "Electronics" }
                    div { class: "making-of-caption", "Electronics" }
                    div { class: "making-of-description",
                        "A Raspberry Pi 3 is controlling the birdhouse and is located on the roof. We coded the backend using Django and fine-tuned the GPIO motion detection with Python. The software can be found on "
                        a { href: "https://github.com/hacknus/birdhouse-monitor", target: "_blank", "Github" }
                        "."
                    }
                }

                // Step 3
                div { class: "making-of-item",
                    img { class: "making-of-image", src: "/outside.jpg", alt: "Complete Assembly" }
                    div { class: "making-of-caption", "Complete Assembly" }
                    div { class: "making-of-description", "Everything was then assembled and is ready to be mounted outside." }
                }

                // Step 4
                div { class: "making-of-item",
                    div { class: "making-of-image-wrap",
                        img { class: "making-of-image rotate-fix", src: "/mounting.jpg", alt: "Mounting Outside" }
                    }
                    div { class: "making-of-caption", "Mounting Outside" }
                    div { class: "making-of-description", "The Birdhouse was then mounted outside with space-grade zip-ties. Power and LAN are passed through thin cables from the office." }
                }

                // Step 5
                div { class: "making-of-item",
                    div { class: "making-of-image-wrap",
                        img { class: "making-of-image rotate-fix", src: "/final.jpg", alt: "Final Birdhouse" }
                    }
                    div { class: "making-of-caption", "Birdhouse" }
                    div { class: "making-of-description", "The birdhouse is shining in all its glory and ready for check in!" }
                }
            }
        }
    }
}
