package bftsmart;

import java.net.*;
import java.io.*;

public abstract class SenderSocket{

//    private static final Logger LOG = LoggerFactory.getLogger(EchoMultiServer.class);

    private static Socket clientSocket;
    private static PrintWriter out;
    private static BufferedReader in;

    public static void init(){

        try {
            clientSocket = new Socket("127.0.0.1", 9437);
            out = new PrintWriter(clientSocket.getOutputStream(), true);
            in = new BufferedReader(new InputStreamReader(clientSocket.getInputStream()));
        } catch (IOException e) {
            e.printStackTrace();
        }

        Thread thread = new Thread(){
            public void run(){
                String line;
                try {
                    while ((line = in.readLine()) != null) {
                        System.out.println("Received: " + line);
                        if (line.equals("done")){
                            System.out.println("Received turn off sinal, shutting down");
                            System.exit(0);
                        }
                    }
                }catch (Exception e){
                    e.printStackTrace();
                }
            }
        };

        thread.start();
    }

    public static void sendMessage(String msg) {
        try {
            out.println(msg);
        }catch (Exception e) {
            e.printStackTrace();
        }
    }

    public static void stopConnection() {
        try {
            in.close();
            out.close();
            clientSocket.close();
        } catch (IOException e) {
            e.printStackTrace();
        }
    }
}