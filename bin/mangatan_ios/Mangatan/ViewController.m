#import "ViewController.h"
#import "Mangatan-Bridging-Header.h"
#import <WebKit/WebKit.h>

@interface ViewController ()
@property (nonatomic, strong) WKWebView *webView;
@property (nonatomic, strong) UIView *loadingView;
@property (nonatomic, strong) NSTimer *statusTimer;
@property (nonatomic, assign) BOOL wasReady;
@end

@implementation ViewController

- (void)viewDidLoad {
    [super viewDidLoad];
    self.wasReady = NO;

    // 1. Setup WebView (Hidden initially)
    self.webView = [[WKWebView alloc] initWithFrame:self.view.bounds];
    self.webView.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    self.webView.scrollView.contentInsetAdjustmentBehavior = UIScrollViewContentInsetAdjustmentNever;
    self.webView.alpha = 0.0;
    [self.view addSubview:self.webView];

    // 2. Setup Loading View
    self.loadingView = [[UIView alloc] initWithFrame:self.view.bounds];
    self.loadingView.backgroundColor = [UIColor systemBackgroundColor];
    self.loadingView.autoresizingMask = UIViewAutoresizingFlexibleWidth | UIViewAutoresizingFlexibleHeight;
    
    UIActivityIndicatorView *spinner = [[UIActivityIndicatorView alloc] initWithActivityIndicatorStyle:UIActivityIndicatorViewStyleLarge];
    spinner.center = self.loadingView.center;
    [spinner startAnimating];
    
    UILabel *label = [[UILabel alloc] initWithFrame:CGRectMake(0, spinner.frame.origin.y + 50, self.view.bounds.size.width, 30)];
    label.text = @"Mangatan is starting...";
    label.textAlignment = NSTextAlignmentCenter;
    
    [self.loadingView addSubview:spinner];
    [self.loadingView addSubview:label];
    [self.view addSubview:self.loadingView];

    // 3. Start Polling Timer (Checks Rust every 1 second)
    self.statusTimer = [NSTimer scheduledTimerWithTimeInterval:1.0
                                                        target:self
                                                      selector:@selector(checkServerStatus)
                                                      userInfo:nil
                                                       repeats:YES];
}

- (void)checkServerStatus {
    BOOL isReady = is_server_ready();
    
    if (isReady && !self.wasReady) {
        // Transition: Server just became ready -> Show Web
        NSLog(@"[UI] Server Ready! Loading interface...");
        [self.webView loadRequest:[NSURLRequest requestWithURL:[NSURL URLWithString:@"http://127.0.0.1:4568"]]];
        
        [UIView animateWithDuration:0.5 animations:^{
            self.webView.alpha = 1.0;
            self.loadingView.alpha = 0.0;
        }];
        self.wasReady = YES;
        
    } else if (!isReady && self.wasReady) {
        // Transition: Server went down -> Show Loading
        NSLog(@"[UI] Server lost! Showing loading screen...");
        [UIView animateWithDuration:0.5 animations:^{
            self.webView.alpha = 0.0;
            self.loadingView.alpha = 1.0;
        }];
        self.wasReady = NO;
    }
}

@end
