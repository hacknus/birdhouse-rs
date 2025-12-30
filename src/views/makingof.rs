use dioxus::prelude::*;

pub fn MakingOf() -> Element {
    rsx! {
        style {
            "
            .making-of-section {{
                max-width: 1200px;
                margin: 0 auto;
                padding: 60px 20px;
            }}
            .making-of-title {{
                font-size: 3rem;
                font-weight: 600;
                text-align: center;
                margin-bottom: 1rem;
                color: #1d1d1f;
            }}
            .making-of-subtitle {{
                font-size: 1.25rem;
                text-align: center;
                color: #6e6e73;
                max-width: 800px;
                margin: 0 auto 60px;
                line-height: 1.5;
            }}
            .making-of-grid {{
                display: grid;
                grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
                gap: 40px;
                margin-bottom: 60px;
            }}
            .making-of-item {{
                text-align: center;
            }}
            .making-of-image {{
                width: 100%;
                height: auto;
                border-radius: 12px;
                margin-bottom: 20px;
            }}
            .making-of-image.rotated {{
                transform: rotate(90deg);
                max-width: 80%;
                margin-left: auto;
                margin-right: auto;
            }}
            .making-of-caption {{
                font-size: 1.1rem;
                font-weight: 600;
                color: #1d1d1f;
                margin-bottom: 8px;
            }}
            .making-of-description {{
                font-size: 0.95rem;
                color: #6e6e73;
                line-height: 1.5;
            }}
            .making-of-description a {{
                color: #0071e3;
                text-decoration: none;
            }}
            .making-of-description a:hover {{
                text-decoration: underline;
            }}
            "
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
                    img { class: "making-of-image rotated", src: "/mounting.jpg", alt: "Mounting Outside" }
                    div { class: "making-of-caption", "Mounting Outside" }
                    div { class: "making-of-description", "The Birdhouse was then mounted outside with space-grade zip-ties. Power and LAN are passed through thin cables from the office." }
                }

                // Step 5
                div { class: "making-of-item",
                    img { class: "making-of-image rotated", src: "/final.jpg", alt: "Final Birdhouse" }
                    div { class: "making-of-caption", "Birdhouse" }
                    div { class: "making-of-description", "The birdhouse is shining in all its glory and ready for check in!" }
                }
            }
        }
    }
}