#manifest
self as example.client

#scenes
"scene"
    FlexNode{
        width:100vw height:100vh
        flex_direction:Column justify_main:Center justify_cross:Center
    }

    "status"
        AbsoluteNode{top:5px right:9px bottom:Auto left:Auto}
        TextLine

    "owner"
        FlexNode{margin:{bottom:25px}}
        TextLine

"button"
    FlexNode{width:300px height:175px}
    Multi<Responsive<BackgroundColor>>[
        {idle:Hsla{hue:125 saturation:0.25 lightness:0.45 alpha:1}}
        {state:[Selected] idle:Hsla{hue:125 saturation:0.4 lightness:0.3 alpha:1}}
    ]
